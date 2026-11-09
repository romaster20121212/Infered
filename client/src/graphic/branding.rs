 
 
 
 
 
 
 
 
 
 

use egui::{Context, TextureHandle, TextureOptions};

 
static LOGO_PNG_BYTES: &[u8] = include_bytes!("../../assets/infered.png");

 
static LOGO_TEX: std::sync::OnceLock<std::sync::Mutex<Option<TextureHandle>>> =
    std::sync::OnceLock::new();

 
static LOGO_SIZE: std::sync::OnceLock<(u32, u32)> = std::sync::OnceLock::new();

 
pub fn logo_size() -> (u32, u32) {
    static FALLBACK: (u32, u32) = (1536, 1024);
    if let Some(size) = LOGO_SIZE.get() {
        return *size;
    }
     
     
    match image::load_from_memory(LOGO_PNG_BYTES) {
        Ok(image) => {
            let size = (image.width(), image.height());
            let _ = LOGO_SIZE.set(size);
            size
        }
        Err(_) => FALLBACK,
    }
}

 
 
 
 
 
 
pub fn brand_mask(image: &image::DynamicImage) -> egui::ColorImage {
    let rgba = image.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let pixels = rgba
        .pixels()
        .map(|pixel| {
            let value = pixel[0].max(pixel[1]).max(pixel[2]);
            egui::Color32::from_rgba_unmultiplied(value, value, value, pixel[3])
        })
        .collect();
    egui::ColorImage { size, pixels }
}

 
 
 
 
pub fn logo_texture(ctx: &Context) -> Option<egui::TextureId> {
    let lock = LOGO_TEX.get_or_init(|| std::sync::Mutex::new(None));
    let mut tex = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if tex.is_none() {
        let image = image::load_from_memory(LOGO_PNG_BYTES).ok()?;
        let size = (image.width(), image.height());
        let _ = LOGO_SIZE.set(size);
        let mask = brand_mask(&image);
        *tex = Some(ctx.load_texture("infered_brand_logo_mask", mask, TextureOptions::LINEAR));
    }
    tex.as_ref().map(|handle| handle.id())
}

 
static SETTINGS_ICON_PNG: &[u8] = include_bytes!("../../../logo's/settings.png");
static SETTINGS_ICON_TEX: std::sync::OnceLock<std::sync::Mutex<Option<TextureHandle>>> =
    std::sync::OnceLock::new();

 
 
 
pub fn settings_icon_texture(ctx: &Context) -> Option<TextureHandle> {
    let lock = SETTINGS_ICON_TEX.get_or_init(|| std::sync::Mutex::new(None));
    let mut tex = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if tex.is_none() {
        let image = image::load_from_memory(SETTINGS_ICON_PNG).ok()?;
        let resized = image::imageops::resize(
            &image.to_rgba8(),
            64,
            64,
            image::imageops::FilterType::Lanczos3,
        );
        let mask = brand_mask(&image::DynamicImage::ImageRgba8(resized));
        *tex = Some(ctx.load_texture("infered_settings_icon_mask", mask, TextureOptions::LINEAR));
    }
    tex.clone()
}

 
 
 
static TITLE_LOGO_PNG: &[u8] = include_bytes!("../../../logo's/infered.png");
static TITLE_LOGO_TEX: std::sync::OnceLock<std::sync::Mutex<Option<TextureHandle>>> =
    std::sync::OnceLock::new();
static TITLE_LOGO_SIZE: std::sync::OnceLock<(u32, u32)> = std::sync::OnceLock::new();

 
 
pub fn title_logo_size() -> (u32, u32) {
    if let Some(size) = TITLE_LOGO_SIZE.get() {
        return *size;
    }
    let size = image::load_from_memory(TITLE_LOGO_PNG)
        .map(|image| (image.width(), image.height()))
        .unwrap_or((1536, 1024));
    let _ = TITLE_LOGO_SIZE.set(size);
    size
}

 
 
pub fn title_logo_texture(ctx: &Context) -> Option<TextureHandle> {
    let lock = TITLE_LOGO_TEX.get_or_init(|| std::sync::Mutex::new(None));
    let mut tex = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if tex.is_none() {
        let image = image::load_from_memory(TITLE_LOGO_PNG).ok()?;
        let size = (image.width(), image.height());
        let _ = TITLE_LOGO_SIZE.set(size);
        let rgba = image.to_rgba8();
        let color = egui::ColorImage::from_rgba_unmultiplied(
            [rgba.width() as usize, rgba.height() as usize],
            rgba.as_raw(),
        );
        *tex = Some(ctx.load_texture("infered_title_logo", color, TextureOptions::LINEAR));
    }
    tex.clone()
}

 
 
static MENU_LOGO_PNG: &[u8] = include_bytes!("../../assets/logo.png");
static MENU_LOGO_TEX: std::sync::OnceLock<std::sync::Mutex<Option<TextureHandle>>> =
    std::sync::OnceLock::new();

 
 
pub fn menu_logo_texture(ctx: &Context) -> Option<TextureHandle> {
    let lock = MENU_LOGO_TEX.get_or_init(|| std::sync::Mutex::new(None));
    let mut tex = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if tex.is_none() {
        let image = image::load_from_memory(MENU_LOGO_PNG).ok()?;
        let mask = brand_mask(&image);
        *tex = Some(ctx.load_texture("infered_menu_logo_mask", mask, TextureOptions::LINEAR));
    }
    tex.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, RgbaImage};

    fn mask_of(pixels: &[[u8; 4]]) -> egui::ColorImage {
        let raw: Vec<u8> = pixels.iter().flatten().copied().collect();
        let rgba = RgbaImage::from_raw(pixels.len() as u32, 1, raw).unwrap();
        brand_mask(&DynamicImage::ImageRgba8(rgba))
    }

    #[test]
    fn red_art_becomes_a_bright_grayscale_mask() {
        let mask = mask_of(&[[225, 0, 0, 255]]);
        assert_eq!(mask.size, [1, 1]);
        assert_eq!(
            mask.pixels[0],
            egui::Color32::from_rgba_unmultiplied(225, 225, 225, 255)
        );
    }

    #[test]
    fn the_title_logo_decodes_at_three_by_two() {
        let size = title_logo_size();
        assert_eq!(size, (1536, 1024));
        assert_eq!(size.0 * 2, size.1 * 3, "the artwork is 3:2");
    }

    #[test]
    fn the_settings_icon_decodes() {
        let image = image::load_from_memory(SETTINGS_ICON_PNG).expect("settings.png is valid");
        assert_eq!((image.width(), image.height()), (1254, 1254));
    }

    #[test]
    fn black_linework_stays_black_and_alpha_is_kept() {
        let mask = mask_of(&[[0, 0, 0, 255], [0, 0, 0, 0], [20, 90, 15, 200]]);
        assert_eq!(
            mask.pixels[0],
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 255)
        );
        assert_eq!(
            mask.pixels[1],
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 0)
        );
         
        assert_eq!(
            mask.pixels[2],
            egui::Color32::from_rgba_unmultiplied(90, 90, 90, 200)
        );
    }
}
