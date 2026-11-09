 
 
 
 
 
 
 
 
 
 
 
 

use std::path::Path;

use anyhow::{ensure, Context};
use image::RgbaImage;
use windows::core::{Interface, HSTRING};
use windows::Win32::Foundation::BOOL;
use windows::Win32::Media::MediaFoundation::{
    IMFMediaBuffer, IMFMediaType, IMFSourceReader, MFCreateMediaType, MFCreateSourceReaderFromURL,
    MFMediaType_Video, MFStartup, MFVideoFormat_RGB32, MFSTARTUP_NOSOCKET, MF_MT_DEFAULT_STRIDE,
    MF_MT_FRAME_RATE, MF_MT_FRAME_SIZE, MF_MT_MAJOR_TYPE, MF_MT_SUBTYPE,
    MF_SOURCE_READER_ALL_STREAMS, MF_SOURCE_READER_FIRST_VIDEO_STREAM, MF_VERSION,
};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

use super::chams_video::{Clip, MAX_FPS, MAX_FRAMES};

 
const END_OF_STREAM: u32 = 0x2;

 
 
fn start() -> anyhow::Result<()> {
    static STARTED: std::sync::OnceLock<anyhow::Result<()>> = std::sync::OnceLock::new();
    let result = STARTED.get_or_init(|| {
         
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        unsafe { MFStartup(MF_VERSION, MFSTARTUP_NOSOCKET) }
            .map_err(|error| anyhow::anyhow!("Media Foundation did not start: {error}"))
    });
    match result {
        Ok(()) => Ok(()),
        Err(error) => anyhow::bail!("{error}"),
    }
}

 
pub(crate) fn decode(path: &Path, fps_hint: u32) -> anyhow::Result<Clip> {
    start()?;
    let text = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned();
    unsafe { read(&text, path, fps_hint) }
}

unsafe fn read(text: &str, path: &Path, fps_hint: u32) -> anyhow::Result<Clip> {
    let reader = MFCreateSourceReaderFromURL(&HSTRING::from(text), None)
        .with_context(|| format!("opening {}", path.display()))?;
    reader
        .SetStreamSelection(MF_SOURCE_READER_ALL_STREAMS.0 as u32, BOOL(0))
        .context("clearing the stream selection")?;
    reader
        .SetStreamSelection(MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32, BOOL(1))
        .context("selecting the video stream")?;

     
     
    let output: IMFMediaType = MFCreateMediaType().context("creating the output type")?;
    output
        .SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)
        .context("setting the output major type")?;
    output
        .SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32)
        .context("setting the output subtype")?;
    reader
        .SetCurrentMediaType(MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32, None, &output)
        .with_context(|| {
            format!(
                "{} has no decoder Media Foundation can use (WebM usually needs the VP9 \
                 extension installed)",
                path.display()
            )
        })?;

    let current = reader
        .GetCurrentMediaType(MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32)
        .context("reading the decoded format")?;
    let size = current.GetUINT64(&MF_MT_FRAME_SIZE).unwrap_or(0);
    let width = (size >> 32) as u32;
    let height = (size & 0xffff_ffff) as u32;
    ensure!(width > 0 && height > 0, "the clip reports no frame size");

    let rate = current.GetUINT64(&MF_MT_FRAME_RATE).unwrap_or(30 << 32);
    let numerator = (rate >> 32) as f32;
    let denominator = (rate & 0xffff_ffff) as f32;
    let source_fps = if numerator > 0.0 && denominator > 0.0 {
        numerator / denominator
    } else {
        30.0
    };
     
    let stride = current
        .GetUINT32(&MF_MT_DEFAULT_STRIDE)
        .map(|value| value as i32)
        .unwrap_or((width * 4) as i32);
     
    let keep_every = ((source_fps / MAX_FPS).ceil() as usize).max(1);

    let mut frames: Vec<(i64, RgbaImage)> = Vec::new();
    let mut seen = 0usize;
    loop {
        let mut flags = 0u32;
        let mut stamp = 0i64;
        let mut sample = None;
        reader
            .ReadSample(
                MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32,
                0,
                None,
                Some(&mut flags),
                Some(&mut stamp),
                Some(&mut sample),
            )
            .with_context(|| format!("reading {}", path.display()))?;
        if flags & END_OF_STREAM != 0 {
            break;
        }
        let Some(sample) = sample else {
            continue;
        };
        seen += 1;
        if (seen - 1) % keep_every != 0 {
            continue;
        }
        if frames.len() >= MAX_FRAMES {
            break;
        }
        let buffer: IMFMediaBuffer = sample
            .ConvertToContiguousBuffer()
            .context("flattening a frame")?;
        let mut data: *mut u8 = std::ptr::null_mut();
        let mut length = 0u32;
        buffer
            .Lock(&mut data, None, Some(&mut length))
            .context("locking a frame")?;
        let picture = convert(data, length, width, height, stride);
        buffer.Unlock().context("unlocking a frame")?;
        match picture {
            Some(picture) => frames.push((stamp, picture)),
            None => anyhow::bail!(
                "{} produced a {width}x{height} frame of {length} bytes, too small to read",
                path.display()
            ),
        }
    }
    ensure!(
        !frames.is_empty(),
        "{} decoded no frames (is the video stream empty?)",
        path.display()
    );

     
     
    frames.sort_by_key(|(stamp, _)| *stamp);
    let timed: Vec<(RgbaImage, u32)> = frames
        .iter()
        .enumerate()
        .map(|(index, (stamp, picture))| {
             
            let next = frames
                .get(index + 1)
                .map(|(next, _)| *next)
                .unwrap_or(stamp + 333_333);
            let delay = ((next - *stamp) / 10_000).clamp(1, 2_000) as u32;
            (picture.clone(), delay)
        })
        .collect();
     
    let _ = fps_hint;
    let fps = (source_fps.round() as u32).clamp(1, 60);
    Clip::build(timed, fps)
}

 
 
fn convert(
    data: *const u8,
    length: u32,
    width: u32,
    height: u32,
    stride: i32,
) -> Option<RgbaImage> {
    let pitch = stride.unsigned_abs() as usize;
    if data.is_null() || width == 0 || height == 0 {
        return None;
    }
     
    let row = (width as usize) * 4;
    if pitch < row || (length as usize) < pitch {
        return None;
    }
    let mut picture = RgbaImage::new(width, height);
    let bottom_up = stride < 0;
    for y in 0..height as usize {
        let row_index = if bottom_up {
            height as usize - 1 - y
        } else {
            y
        };
        let start = row_index * pitch;
        if start + row > length as usize {
            return None;
        }
         
        let source = unsafe { std::slice::from_raw_parts(data.add(start), row) };
        for x in 0..width as usize {
            let at = x * 4;
            picture.put_pixel(
                x as u32,
                y as u32,
                image::Rgba([source[at + 2], source[at + 1], source[at], source[at + 3]]),
            );
        }
    }
    Some(picture)
}

 
#[allow(dead_code)]
fn _interfaces() {
    fn assert_interface<T: Interface>() {}
    assert_interface::<IMFSourceReader>();
    assert_interface::<IMFMediaType>();
    assert_interface::<IMFMediaBuffer>();
}
