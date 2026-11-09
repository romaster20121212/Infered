 
 
 
 
 
 
 
 
 
 
 

use egui::{Context, FontData, FontDefinitions, FontFamily, FontId};
use std::sync::atomic::{AtomicUsize, Ordering};

 
 
 
const UI_FONTS: &[(&str, &[u8])] = &[
    (
        "Audiowide",
        include_bytes!("../../../assets/ui-fonts/Audiowide-Regular.ttf"),
    ),
    (
        "Russo One",
        include_bytes!("../../../assets/ui-fonts/RussoOne-Regular.ttf"),
    ),
    (
        "VT323",
        include_bytes!("../../../assets/ui-fonts/VT323-Regular.ttf"),
    ),
];

 
const UI_FONT_KEY: &str = "infered-ui-font";

 
static UI_FONT: AtomicUsize = AtomicUsize::new(0);

 
const RIGHTEOUS: &[u8] = include_bytes!("../../../assets/fonts/Righteous-Regular.ttf");

 
pub(crate) const LYRICS_FAMILY: &str = "infered-lyrics";
 
const LYRICS_FONT_KEY: &str = "infered-lyrics-display";

 
pub(crate) fn lyrics_family() -> FontFamily {
    FontFamily::Name(LYRICS_FAMILY.into())
}

 
pub(crate) fn lyrics_font(size: f32) -> FontId {
    FontId::new(size, lyrics_family())
}

 
 
pub(crate) fn ui_font_names() -> Vec<&'static str> {
    std::iter::once("Default")
        .chain(UI_FONTS.iter().map(|(name, _)| *name))
        .collect()
}

 
pub(crate) fn ui_font() -> usize {
    UI_FONT.load(Ordering::Relaxed).min(UI_FONTS.len())
}

 
 
pub(crate) fn set_ui_font(index: usize) {
    UI_FONT.store(index.min(UI_FONTS.len()), Ordering::Relaxed);
}

 
pub(crate) fn apply_ui_font(ctx: &Context) {
    install(ctx);
}

 
 
pub(crate) fn install(ctx: &Context) {
    let mut fonts = FontDefinitions::default();

     
     
    let chosen = UI_FONT.load(Ordering::Relaxed);
    if chosen > 0 {
        fonts.font_data.insert(
            UI_FONT_KEY.to_owned(),
            FontData::from_static(UI_FONTS[chosen - 1].1),
        );
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            fonts
                .families
                .entry(family)
                .or_default()
                .insert(0, UI_FONT_KEY.to_owned());
        }
    }
    fonts
        .font_data
        .insert(LYRICS_FONT_KEY.to_owned(), FontData::from_static(RIGHTEOUS));
    fonts.families.insert(
        lyrics_family(),
        vec![
            LYRICS_FONT_KEY.to_owned(),
             
             
             
            "Ubuntu-Light".to_owned(),
            "NotoEmoji-Regular".to_owned(),
            "emoji-icon-font".to_owned(),
        ],
    );
    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Color32, Context};

     
     
    fn installed_context() -> Context {
        let ctx = Context::default();
        install(&ctx);
        let _ = ctx.run(Default::default(), |_| {});
        ctx
    }

    fn width(ctx: &Context, text: &str) -> f32 {
        ctx.fonts(|fonts| {
            fonts
                .layout_no_wrap(text.to_owned(), lyrics_font(24.0), Color32::WHITE)
                .size()
                .x
        })
    }

    #[test]
    fn the_display_family_is_registered_and_lays_out_text() {
        let ctx = installed_context();
        assert!(width(&ctx, "hello world") > 0.0);
         
        assert_eq!(LYRICS_FAMILY, "infered-lyrics");
        assert_eq!(lyrics_family(), FontFamily::Name(LYRICS_FAMILY.into()));
    }

    #[test]
    fn missing_glyphs_fall_back_instead_of_vanishing() {
        let ctx = installed_context();
         
        assert!(
            width(&ctx, "♪") > 0.0,
            "the instrumental marker must render through a fallback"
        );
    }

    #[test]
    fn the_family_is_a_display_face_not_the_default() {
        let ctx = installed_context();
        let display = width(&ctx, "Now the words hang in the air");
        let default = ctx.fonts(|fonts| {
            fonts
                .layout_no_wrap(
                    "Now the words hang in the air".to_owned(),
                    FontId::proportional(24.0),
                    Color32::WHITE,
                )
                .size()
                .x
        });
        assert!(display > 0.0 && default > 0.0);
        assert!(
            (display - default).abs() > 0.5,
            "the lyrics family must not silently resolve to the UI font"
        );
    }
}
