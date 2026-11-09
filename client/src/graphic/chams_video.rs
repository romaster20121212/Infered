 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{ensure, Context};
use image::AnimationDecoder;

use crate::gl;

 
pub(crate) const FOLDER: &str = "chams_videos";

 
 
pub(crate) const STYLE: i32 = 4;

 
 
const MAX_SIDE: u32 = 160;

 
pub(crate) const MAX_FRAMES: usize = 240;

 
 
pub(crate) const MAX_FPS: f32 = 30.0;

 
 
const CLIP_EXTENSIONS: [&str; 9] = [
    "gif", "webp", "apng", "png", "jpg", "jpeg", "mp4", "webm", "mov",
];

 
pub(crate) struct Clip {
    width: u32,
    height: u32,
     
    frames: Vec<Vec<u8>>,
     
    stamps: Vec<f32>,
    length: f32,
}

impl std::fmt::Debug for Clip {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Clip")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("frames", &self.frames.len())
            .field("length", &self.length)
            .finish()
    }
}

impl Clip {
    pub(crate) fn width(&self) -> u32 {
        self.width
    }

    pub(crate) fn height(&self) -> u32 {
        self.height
    }

    pub(crate) fn frames(&self) -> usize {
        self.frames.len()
    }

    pub(crate) fn length(&self) -> f32 {
        self.length
    }

     
     
     
    pub(crate) fn frame_at(&self, time: f32) -> (usize, &[u8]) {
        let index = self.index_at(time);
        (index, &self.frames[index])
    }

    fn index_at(&self, time: f32) -> usize {
        if self.frames.len() < 2 || !(self.length > 0.0) || !time.is_finite() {
            return 0;
        }
        let at = time.rem_euclid(self.length);
         
        match self.stamps.binary_search_by(|stamp| stamp.total_cmp(&at)) {
            Ok(index) => index.min(self.frames.len() - 1),
            Err(0) => 0,
            Err(index) => (index - 1).min(self.frames.len() - 1),
        }
    }

     
     
    pub(crate) fn build(frames: Vec<(image::RgbaImage, u32)>, fps: u32) -> anyhow::Result<Self> {
        ensure!(!frames.is_empty(), "the clip has no frames");
        let fallback = (1000.0 / fps.clamp(1, 60) as f32).max(1.0);
        let total = frames.len();
        let stride = ((total as f32 / MAX_FRAMES as f32).ceil() as usize).max(1);
        let mut kept: Vec<(image::RgbaImage, u32)> = Vec::new();
        for (index, frame) in frames.into_iter().enumerate() {
             
            if index % stride == 0 {
                kept.push(frame);
            }
        }
        let (mut width, mut height) = kept[0].0.dimensions();
        ensure!(width > 0 && height > 0, "the first frame is empty");
        let scale = (MAX_SIDE as f32 / width.max(height) as f32).min(1.0);
        width = ((width as f32 * scale).round() as u32).max(1);
        height = ((height as f32 * scale).round() as u32).max(1);

        let mut pixels = Vec::with_capacity(kept.len());
        let mut stamps = Vec::with_capacity(kept.len());
        let mut at = 0.0f32;
        for (frame, delay) in kept {
            let resized = if frame.dimensions() == (width, height) {
                frame
            } else {
                image::imageops::resize(
                    &frame,
                    width,
                    height,
                    image::imageops::FilterType::Triangle,
                )
            };
            stamps.push(at);
            pixels.push(resized.into_raw());
            let seconds = if delay == 0 { fallback } else { delay as f32 } / 1000.0;
             
            at += seconds.clamp(0.01, 2.0);
        }
        Ok(Self {
            width,
            height,
            frames: pixels,
            stamps,
            length: at,
        })
    }
}

 
 
pub(crate) fn load(path: &Path, fps: u32) -> anyhow::Result<Clip> {
    ensure!(path.exists(), "{} does not exist", path.display());
    if path.is_dir() {
        return sequence(path, fps);
    }
    let extension = extension(path);
    match extension.as_str() {
        "gif" | "webp" => match animated(path, fps) {
            Ok(clip) => Ok(clip),
             
             
            Err(animated_error) => still(path).with_context(|| {
                format!(
                    "{} is neither an animation nor a picture: {animated_error:#}",
                    path.display()
                )
            }),
        },
        "png" | "apng" | "jpg" | "jpeg" => still(path),
        "mp4" | "webm" | "mov" => movie(path, fps),
        other => anyhow::bail!(
            "{} is not a clip (found .{other}; use gif, webp, apng, png, jpg, mp4 or webm)",
            path.display()
        ),
    }
}

fn extension(path: &Path) -> String {
    path.extension()
        .map(|value| value.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

 
 
fn animated(path: &Path, fps: u32) -> anyhow::Result<Clip> {
    let file = std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let reader = std::io::BufReader::new(file);
    let extension = extension(path);
    let decoded = match extension.as_str() {
        "gif" => image::codecs::gif::GifDecoder::new(reader)?
            .into_frames()
            .collect_frames(),
        _ => image::codecs::webp::WebPDecoder::new(reader)?
            .into_frames()
            .collect_frames(),
    }
    .with_context(|| format!("decoding the frames of {}", path.display()))?;
    let frames = decoded
        .into_iter()
        .map(|frame| {
            let (numerator, denominator) = frame.delay().numer_denom_ms();
            let delay = if denominator == 0 {
                0
            } else {
                numerator / denominator
            };
            (frame.into_buffer(), delay)
        })
        .collect::<Vec<_>>();
    Clip::build(frames, fps)
}

 
fn still(path: &Path) -> anyhow::Result<Clip> {
    let picture = image::ImageReader::open(path)
        .with_context(|| format!("opening {}", path.display()))?
        .with_guessed_format()
        .with_context(|| format!("reading {}", path.display()))?
        .decode()
        .with_context(|| format!("decoding {}", path.display()))?
        .to_rgba8();
     
    Clip::build(vec![(picture, 0)], 30)
}

 
fn sequence(folder: &Path, fps: u32) -> anyhow::Result<Clip> {
    let files = frame_files(folder);
    ensure!(
        !files.is_empty(),
        "{} holds no png or jpg frames",
        folder.display()
    );
    let delay = (1000.0 / fps.clamp(1, 60) as f32).round() as u32;
    let mut frames = Vec::with_capacity(files.len());
    for file in files.iter().take(MAX_FRAMES) {
        let picture = image::ImageReader::open(file)
            .with_context(|| format!("opening {}", file.display()))?
            .with_guessed_format()
            .with_context(|| format!("reading {}", file.display()))?
            .decode()
            .with_context(|| format!("decoding {}", file.display()))?
            .to_rgba8();
        frames.push((picture, delay));
    }
    Clip::build(frames, fps)
}

 
fn frame_files(folder: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(folder)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| {
                    path.is_file() && matches!(extension(path).as_str(), "png" | "jpg" | "jpeg")
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

 
 
#[cfg(not(target_os = "windows"))]
fn movie(path: &Path, _fps: u32) -> anyhow::Result<Clip> {
    anyhow::bail!(
        "{} needs Media Foundation, which this build does not have — drop an animated \
         GIF/WebP or a folder of frames into {FOLDER} instead",
        path.display()
    )
}

#[cfg(target_os = "windows")]
fn movie(path: &Path, fps: u32) -> anyhow::Result<Clip> {
    super::chams_movie::decode(path, fps)
}

 
 
pub(crate) fn clip_files(folder: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(folder)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| {
                    if path.is_dir() {
                        !frame_files(path).is_empty()
                    } else {
                        path.is_file() && CLIP_EXTENSIONS.contains(&extension(path).as_str())
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

 
 
 
 
 
pub(crate) fn clip_names(files: &[PathBuf]) -> Vec<String> {
    let stems: Vec<String> = files.iter().map(|path| clip_name(path)).collect();
    files
        .iter()
        .enumerate()
        .map(|(index, path)| {
            let stem = &stems[index];
            let shared = stems
                .iter()
                .enumerate()
                .any(|(other, name)| other != index && name == stem);
            if shared {
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| stem.clone())
            } else {
                stem.clone()
            }
        })
        .collect()
}

 
pub(crate) fn clip_name(path: &Path) -> String {
    path.file_stem()
        .or_else(|| path.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

 
 
 
pub(crate) fn folder() -> PathBuf {
    crate::config::base_dir().join(FOLDER)
}

 
fn writable_folder() -> anyhow::Result<PathBuf> {
    let folder = folder();
    std::fs::create_dir_all(&folder).with_context(|| format!("creating {}", folder.display()))?;
    Ok(folder)
}

 
 
 
pub(crate) fn path_for_label(label: &str) -> Option<PathBuf> {
    path_for_label_in(&folder(), label)
}

 
 
fn path_for_label_in(folder: &Path, label: &str) -> Option<PathBuf> {
    let files = clip_files(folder);
    let names = clip_names(&files);
    files
        .into_iter()
        .zip(names)
        .find_map(|(path, name)| (name == label).then_some(path))
}

 
 
 
pub(crate) fn import(source: &Path) -> anyhow::Result<PathBuf> {
    import_into(&writable_folder()?, source)
}

 
fn import_into(folder: &Path, source: &Path) -> anyhow::Result<PathBuf> {
    ensure!(source.is_file(), "{} is not a file", source.display());
    let extension = extension(source);
    ensure!(
        CLIP_EXTENSIONS.contains(&extension.as_str()),
        "{} is not a clip (use gif, webp, apng, png, jpg, mp4 or webm)",
        source.display()
    );
     
     
     
     
    let picked = std::fs::canonicalize(source).unwrap_or_else(|_| source.to_path_buf());
    let home = std::fs::canonicalize(folder).unwrap_or_else(|_| folder.to_path_buf());
    let name = picked
        .file_name()
        .map(PathBuf::from)
        .unwrap_or_else(|| picked.clone());
    if picked.parent() == Some(home.as_path()) {
        return Ok(folder.join(name));
    }
    let destination = free_name(folder, &picked);
    std::fs::copy(&picked, &destination).with_context(|| {
        format!(
            "copying {} into {} (is the file in use?)",
            picked.display(),
            folder.display()
        )
    })?;
    Ok(destination)
}

 
 
fn free_name(folder: &Path, source: &Path) -> PathBuf {
    let stem = source
        .file_stem()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "clip".into());
    let extension = extension(source);
    let named = |stem: &str| {
        if extension.is_empty() {
            folder.join(stem)
        } else {
            folder.join(format!("{stem}.{extension}"))
        }
    };
    let first = named(&stem);
    if !first.exists() {
        return first;
    }
    (2..)
        .map(|copy| named(&format!("{stem} ({copy})")))
        .find(|candidate| !candidate.exists())
        .unwrap_or(first)
}

 
 
pub(crate) fn pick() -> anyhow::Result<Option<PathBuf>> {
    let Some(source) = choose_file() else {
        return Ok(None);
    };
    import(&source).map(Some)
}

 
#[cfg(target_os = "windows")]
fn choose_file() -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::UI::Controls::Dialogs::{
        GetOpenFileNameW, OFN_FILEMUSTEXIST, OFN_NOCHANGEDIR, OFN_PATHMUSTEXIST, OPENFILENAMEW,
    };

     
     
    let filter: Vec<u16> = "Clips (*.gif;*.webp;*.mp4;*.webm;*.png;*.jpg)\0\
                            *.gif;*.webp;*.apng;*.mp4;*.webm;*.mov;*.png;*.jpg;*.jpeg\0\
                            All files (*.*)\0*.*\0\0"
        .encode_utf16()
        .collect();
    let mut file = vec![0u16; 1024];
    let mut dialog: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    dialog.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    dialog.lpstrFilter = PCWSTR(filter.as_ptr());
    dialog.nFilterIndex = 1;
    dialog.lpstrFile = PWSTR(file.as_mut_ptr());
    dialog.nMaxFile = file.len() as u32;
    dialog.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;
     
     
    if !unsafe { GetOpenFileNameW(&mut dialog) }.as_bool() {
        return None;
    }
    let len = file
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(file.len());
    Some(PathBuf::from(std::ffi::OsString::from_wide(&file[..len])))
}

 
 
#[cfg(not(target_os = "windows"))]
fn choose_file() -> Option<PathBuf> {
    None
}

 
 
static SELECTED: Mutex<Option<Arc<Clip>>> = Mutex::new(None);

 
static UPLOADED: Mutex<Option<Uploaded>> = Mutex::new(None);

struct Uploaded {
    texture: u32,
     
    clip: usize,
    frame: usize,
}

 
pub(crate) fn select(clip: Option<Arc<Clip>>) {
    *SELECTED.lock().unwrap_or_else(|p| p.into_inner()) = clip;
}

 
pub(crate) fn selected() -> Option<Arc<Clip>> {
    SELECTED
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .as_ref()
        .map(Arc::clone)
}

 
 
 
 
 
pub(crate) unsafe fn texture_at(time: f32) -> Option<u32> {
    let clip = selected()?;
    let (frame, pixels) = clip.frame_at(time);
    let identity = Arc::as_ptr(&clip) as usize;
    let mut uploaded = UPLOADED.lock().unwrap_or_else(|p| p.into_inner());
    let texture = match uploaded.as_ref() {
        Some(current) if current.clip == identity => current.texture,
        _ => {
             
             
            let texture = uploaded.as_ref().map_or_else(
                || {
                    let mut texture = 0;
                    gl::GenTextures(1, &mut texture);
                    texture
                },
                |current| current.texture,
            );
            *uploaded = Some(Uploaded {
                texture,
                clip: identity,
                 
                frame: usize::MAX,
            });
            texture
        }
    };
    let current = uploaded.as_mut()?;
    if current.frame != frame {
        gl::BindTexture(gl::TEXTURE_2D, texture);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA as i32,
            clip.width() as i32,
            clip.height() as i32,
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            pixels.as_ptr().cast(),
        );
        current.frame = frame;
    }
    gl::BindTexture(gl::TEXTURE_2D, texture);
    Some(texture)
}

 
 
pub(crate) unsafe fn release() {
    let mut uploaded = UPLOADED.lock().unwrap_or_else(|p| p.into_inner());
    if let Some(current) = uploaded.take() {
        gl::DeleteTextures(1, &current.texture);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ScratchDir(PathBuf);

    impl ScratchDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("infered_chams_video_{name}_{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for ScratchDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn solid(width: u32, height: u32, [r, g, b, a]: [u8; 4]) -> image::RgbaImage {
        image::RgbaImage::from_pixel(width, height, image::Rgba([r, g, b, a]))
    }

    #[test]
    fn frames_advance_with_their_own_delays_and_wrap() {
        let frames = vec![
            (solid(4, 4, [255, 0, 0, 255]), 100),
            (solid(4, 4, [0, 255, 0, 255]), 200),
            (solid(4, 4, [0, 0, 255, 255]), 300),
        ];
        let clip = Clip::build(frames, 24).unwrap();
        assert_eq!(clip.frames(), 3);
        assert!((clip.length() - 0.6).abs() < 1e-4);
        assert_eq!(clip.index_at(0.0), 0);
        assert_eq!(clip.index_at(0.099), 0);
        assert_eq!(clip.index_at(0.1), 1);
        assert_eq!(clip.index_at(0.35), 2);
         
        assert_eq!(clip.index_at(0.6), 0);
         
        assert_eq!(clip.index_at(1.35), 1);
        assert_eq!(clip.index_at(f32::NAN), 0);
        assert_eq!(clip.index_at(-0.05), 2, "negative time wraps backwards");
    }

    #[test]
    fn a_frame_without_a_delay_uses_the_fps_and_a_long_one_is_capped() {
        let frames = vec![
            (solid(2, 2, [0, 0, 0, 255]), 0),
            (solid(2, 2, [0, 0, 0, 255]), 90_000),
        ];
         
        let clip = Clip::build(frames, 60).unwrap();
        assert!((clip.length() - (1.0 / 60.0 + 2.0)).abs() < 1e-3);
        assert_eq!(clip.index_at(0.001), 0);
        assert_eq!(clip.index_at(0.02), 1);
    }

    #[test]
    fn a_clip_is_downscaled_and_thinned_to_the_memory_bound() {
        let frames: Vec<_> = (0..MAX_FRAMES * 2 + 7)
            .map(|index| (solid(640, 480, [index as u8, 0, 0, 255]), 40))
            .collect();
        let clip = Clip::build(frames, 24).unwrap();
        assert_eq!(clip.width(), MAX_SIDE);
        assert_eq!(clip.height(), 120, "the aspect ratio is kept");
        assert!(clip.frames() <= MAX_FRAMES + 1, "{}", clip.frames());
         
        assert_eq!(clip.frame_at(0.0).1.len(), (MAX_SIDE * 120 * 4) as usize);
    }

    #[test]
    fn a_frame_folder_plays_at_the_requested_rate() {
        let scratch = ScratchDir::new("sequence");
        for (index, [r, g, b]) in [[255u8, 0, 0], [0, 255, 0], [0, 0, 255]]
            .into_iter()
            .enumerate()
        {
            solid(8, 8, [r, g, b, 255])
                .save(scratch.0.join(format!("frame_{index:03}.png")))
                .unwrap();
        }
         
        std::fs::write(scratch.0.join("notes.txt"), b"x").unwrap();
        std::fs::create_dir_all(scratch.0.join("nested")).unwrap();

        let clip = load(&scratch.0, 10).unwrap();
        assert_eq!(clip.frames(), 3);
        assert!((clip.length() - 0.3).abs() < 1e-3);
        assert_eq!(clip.index_at(0.0), 0);
        assert_eq!(clip.index_at(0.1), 1);
        assert_eq!(clip.index_at(0.25), 2);
         
        assert_eq!(clip.frame_at(0.15).1[..4], [0, 255, 0, 255]);
    }

    #[test]
    fn an_animated_gif_loads_every_frame() {
        let scratch = ScratchDir::new("gif");
        let path = scratch.0.join("pulse.gif");
        let file = std::fs::File::create(&path).unwrap();
        let mut encoder = image::codecs::gif::GifEncoder::new(file);
        encoder
            .set_repeat(image::codecs::gif::Repeat::Infinite)
            .unwrap();
        let frames: Vec<image::Frame> = [[255u8, 0, 0], [0, 255, 0], [0, 0, 255]]
            .into_iter()
            .map(|[r, g, b]| {
                image::Frame::from_parts(
                    solid(16, 16, [r, g, b, 255]),
                    0,
                    0,
                    image::Delay::from_numer_denom_ms(100, 1),
                )
            })
            .collect();
        encoder.encode_frames(frames).unwrap();
        drop(encoder);

        let clip = load(&path, 24).unwrap();
        assert_eq!(clip.frames(), 3);
        assert!((clip.length() - 0.3).abs() < 1e-3);
        assert_eq!(clip.frame_at(0.0).1[..3], [255, 0, 0]);
        assert_eq!(clip.frame_at(0.2).1[..3], [0, 0, 255]);
    }

    #[test]
    fn a_still_picture_is_a_one_frame_clip() {
        let scratch = ScratchDir::new("still");
        let path = scratch.0.join("quiet.png");
        solid(20, 10, [10, 20, 30, 255]).save(&path).unwrap();
        let clip = load(&path, 24).unwrap();
        assert_eq!(clip.frames(), 1);
        assert_eq!(clip.frame_at(4.2).0, 0);
        assert_eq!((clip.width(), clip.height()), (20, 10));
    }

    #[test]
    fn unusable_files_explain_themselves() {
        let scratch = ScratchDir::new("errors");
        let text = scratch.0.join("notes.txt");
        std::fs::write(&text, b"x").unwrap();
        let error = load(&text, 24).unwrap_err().to_string();
        assert!(error.contains("is not a clip"), "{error}");

        let empty = scratch.0.join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        let error = load(&empty, 24).unwrap_err().to_string();
        assert!(error.contains("no png or jpg frames"), "{error}");

        let missing = scratch.0.join("absent.gif");
        assert!(load(&missing, 24).is_err());
    }

     
     
    #[test]
    fn clip_files_lists_clips_and_frame_folders_only() {
        let scratch = ScratchDir::new("listing");
        for name in ["b.gif", "A.MP4", "notes.txt", "c.webm", "d.png"] {
            std::fs::write(scratch.0.join(name), b"x").unwrap();
        }
        let frames = scratch.0.join("shot");
        std::fs::create_dir_all(&frames).unwrap();
        solid(2, 2, [0, 0, 0, 255])
            .save(frames.join("01.png"))
            .unwrap();
        std::fs::create_dir_all(scratch.0.join("useless")).unwrap();

        let files = clip_files(&scratch.0);
        assert_eq!(
            clip_names(&files),
            vec!["A", "b", "c", "d", "shot"],
            "{files:?}"
        );
        assert!(clip_files(&scratch.0.join("absent")).is_empty());
    }

     
     
    #[test]
    fn importing_copies_a_clip_into_the_folder() {
        let source = ScratchDir::new("upload_source");
        let folder = ScratchDir::new("upload_folder");
        let picked = source.0.join("cat.gif");
        solid(4, 4, [255, 0, 0, 255]).save(&picked).unwrap();

        let stored = import_into(&folder.0, &picked).unwrap();
        assert_eq!(stored, folder.0.join("cat.gif"));
        assert_eq!(
            std::fs::read(&stored).unwrap(),
            std::fs::read(&picked).unwrap()
        );

         
        let other = source.0.join("again");
        std::fs::create_dir_all(&other).unwrap();
        let second = other.join("cat.gif");
        solid(4, 4, [0, 255, 0, 255]).save(&second).unwrap();
        let stored_second = import_into(&folder.0, &second).unwrap();
        assert_eq!(stored_second, folder.0.join("cat (2).gif"));
         
        assert_eq!(clip_names(&clip_files(&folder.0)), vec!["cat (2)", "cat"]);

         
        let again = import_into(&folder.0, &stored).unwrap();
        assert_eq!(again, stored);
        assert_eq!(clip_files(&folder.0).len(), 2);
    }

    #[test]
    fn importing_rejects_what_is_not_a_clip() {
        let folder = ScratchDir::new("upload_reject");
        let notes = folder.0.join("notes.txt");
        std::fs::write(&notes, b"x").unwrap();
        let error = import_into(&folder.0, &notes).unwrap_err().to_string();
        assert!(error.contains("is not a clip"), "{error}");

        let missing = folder.0.join("absent.gif");
        assert!(import_into(&folder.0, &missing).is_err());
    }

     
     
    #[test]
    fn clips_sharing_a_stem_are_named_apart() {
        let scratch = ScratchDir::new("shared");
        for name in ["cat.gif", "cat.mp4"] {
            std::fs::write(scratch.0.join(name), b"x").unwrap();
        }
        let files = clip_files(&scratch.0);
        assert_eq!(clip_names(&files), vec!["cat.gif", "cat.mp4"]);
        assert_eq!(
            path_for_label_in(&scratch.0, "cat.mp4"),
            Some(scratch.0.join("cat.mp4"))
        );
    }

     
     
     
    #[test]
    fn a_clip_is_found_by_name_rather_than_by_position() {
        let folder = ScratchDir::new("labels");
        for name in ["cat.gif", "dog.gif"] {
            solid(4, 4, [0, 0, 0, 255])
                .save(folder.0.join(name))
                .unwrap();
        }
        assert_eq!(
            path_for_label_in(&folder.0, "dog"),
            Some(folder.0.join("dog.gif"))
        );
        assert_eq!(path_for_label_in(&folder.0, "absent"), None);

        std::fs::remove_file(folder.0.join("dog.gif")).unwrap();
        assert_eq!(path_for_label_in(&folder.0, "dog"), None);
    }

     
     
    #[test]
    fn the_selection_is_the_clip_the_renderer_draws() {
        let clip = Arc::new(Clip::build(vec![(solid(2, 2, [1, 2, 3, 255]), 100)], 24).unwrap());
        select(Some(Arc::clone(&clip)));
        let drawn = selected().expect("a clip is selected");
        assert_eq!(drawn.frames(), 1);
        assert_eq!(drawn.width(), 2);
        select(None);
        assert!(selected().is_none());
    }
}
