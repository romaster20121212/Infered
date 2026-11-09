 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::module::{ModuleData, ModuleSetting};

 
 
pub(crate) const FIRST_STYLE: usize = 5;

 
pub(crate) const MODE_NAMES: [&str; 11] = [
    "Glow",
    "Blur",
    "Galaxy",
    "Prism",
    "Neon",
    "Scan",
    "Liquid Glass",
    "Outline",
    "Contour",
    "Cel",
    "Pulse",
];
pub(crate) const MODE_COUNT: usize = MODE_NAMES.len();

 
pub(crate) const LAST_STYLE: usize = FIRST_STYLE + MODE_COUNT - 1;

 
 
 
pub(crate) const ROWS: [&str; 19] = [
    "Glow Radius",
    "Intensity",
    "Fill Opacity",
    "Outline",
    "Outline Width",
    "Blur",
    "Tint Amount",
    "Speed",
    "Hairline",
    "Thickness",
    "Refraction",
    "Dispersion",
    "Frost",
    "Glass Tint",
    "Lines",
    "Cel Opacity",
    "Reactivity",
    "Idle Beats",
    "Audio Sensitivity",
];

 
 
pub(crate) const REFERENCE_HEIGHT: f32 = 1080.0;

 
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Settings {
     
    pub mode: usize,
    pub color: [f32; 3],

    pub glow_radius: f32,   
    pub intensity: f32,     
    pub fill_opacity: f32,  
    pub outline: bool,      
    pub outline_width: f32,
    pub blur: f32,         
    pub tint_amount: f32,  
    pub speed: f32,        
    pub hairline: bool,    
    pub thickness: f32,    
    pub refraction: f32,   
    pub dispersion: f32,   
    pub frost: f32,        
    pub glass_tint: f32,   
    pub lines: f32,        
    pub cel_opacity: f32,  
    pub reactivity: f32,   
    pub idle_beats: bool,  
    pub audio_sensitivity: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: 0,
             
             
            color: [138.0 / 255.0, 126.0 / 255.0, 1.0],
            glow_radius: 12.0,
            intensity: 1.0,
            fill_opacity: 1.0,
            outline: true,
            outline_width: 2.5,
            blur: 10.0,
            tint_amount: 0.35,
            speed: 1.0,
            hairline: true,
            thickness: 14.0,
            refraction: 1.0,
            dispersion: 0.25,
            frost: 0.0,
            glass_tint: 0.25,
            lines: 6.0,
            cel_opacity: 0.9,
            reactivity: 0.6,
            idle_beats: true,
            audio_sensitivity: 1.0,
        }
    }
}

 
pub(crate) fn style_index(data: &ModuleData) -> usize {
    match data.get_setting("Style") {
        Some(ModuleSetting::Choice { value, options, .. }) => {
            (*value).min(options.len().saturating_sub(1))
        }
        _ => 0,
    }
}

 
pub(crate) fn is_warped(data: &ModuleData) -> bool {
    data.get_setting("Outline Glow")
        .and_then(ModuleSetting::get_toggle_value)
        .unwrap_or(true)
}

 
pub(crate) fn selected_mode(data: &ModuleData) -> Option<usize> {
    let style = style_index(data);
    Some(if style < FIRST_STYLE {
         
        [4, 9, 5, 10, 0][style.min(4)]
    } else {
        (style - FIRST_STYLE).min(MODE_COUNT - 1)
    })
}

 
 
 
pub(crate) fn classic_style(style: i32) -> i32 {
    if style >= 0 && (style as usize) < FIRST_STYLE {
        style
    } else {
        0
    }
}

impl Settings {
     
     
    pub(crate) fn read(data: &ModuleData) -> Self {
        let defaults = Self::default();
        let toggle = |name, fallback| {
            data.get_setting(name)
                .and_then(ModuleSetting::get_toggle_value)
                .unwrap_or(fallback)
        };
        let slider = |name, fallback| {
            data.get_setting(name)
                .and_then(ModuleSetting::get_slider_value)
                .filter(|value| value.is_finite())
                .unwrap_or(fallback)
        };
        let color = match data.get_setting("Color") {
            Some(ModuleSetting::Color { value, .. }) => [value[0], value[1], value[2]],
            _ => [defaults.color[0], defaults.color[1], defaults.color[2]],
        };
        Self {
            mode: selected_mode(data).unwrap_or(defaults.mode),
            color,
            glow_radius: slider("Glow Radius", defaults.glow_radius),
            intensity: slider("Intensity", defaults.intensity),
            fill_opacity: slider("Fill Opacity", defaults.fill_opacity),
            outline: toggle("Outline", defaults.outline),
            outline_width: slider("Outline Width", defaults.outline_width),
            blur: slider("Blur", defaults.blur),
            tint_amount: slider("Tint Amount", defaults.tint_amount),
            speed: slider("Speed", defaults.speed),
            hairline: toggle("Hairline", defaults.hairline),
            thickness: slider("Thickness", defaults.thickness),
            refraction: slider("Refraction", defaults.refraction),
            dispersion: slider("Dispersion", defaults.dispersion),
            frost: slider("Frost", defaults.frost),
            glass_tint: slider("Glass Tint", defaults.glass_tint),
            lines: slider("Lines", defaults.lines),
            cel_opacity: slider("Cel Opacity", defaults.cel_opacity),
            reactivity: slider("Reactivity", defaults.reactivity),
            idle_beats: toggle("Idle Beats", defaults.idle_beats),
            audio_sensitivity: slider("Audio Sensitivity", defaults.audio_sensitivity),
        }
    }

     
    pub(crate) fn mode_name(&self) -> &'static str {
        MODE_NAMES[self.mode.min(MODE_COUNT - 1)]
    }
}

 
 
 

 
 
 
 
 
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Style {
    pub fill: f32,        
    pub rim: f32,         
    pub halo: f32,        
    pub halo_width: f32,  
    pub rim_width: f32,   
    pub inner_glow: f32,  
    pub lines: i32,       
    pub pattern_speed: f32,
    pub ghost: f32,         
    pub dispersion: f32,    
    pub frost: f32,         
    pub glass: f32,         
    pub cel: f32,           
    pub hue_spread: f32,    
    pub star_density: f32,  
    pub tint_mix: f32,      
    pub hairline: bool,
    pub rim_only: bool,  
    pub banded: bool,    
    pub double_rim: bool,
}

pub(crate) fn clamp01(value: f32) -> f32 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, 1.0)
    }
}

pub(crate) fn clamp(value: f32, low: f32, high: f32) -> f32 {
    if value.is_nan() {
        low
    } else {
        value.max(low).min(high)
    }
}

pub(crate) fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * clamp01(t)
}

 
 
pub(crate) fn compute_style(settings: &Settings) -> Style {
    let mut style = Style {
        tint_mix: clamp01(settings.tint_amount),
        rim_width: clamp(settings.outline_width, 0.5, 12.0),
        lines: (clamp(settings.lines, 2.0, 16.0).round() as i32).clamp(2, 16),
        pattern_speed: clamp(settings.speed, 0.0, 5.0),
        dispersion: clamp01(settings.dispersion) * 4.0,
        frost: clamp(settings.frost, 0.0, 20.0) / 20.0,
        hairline: settings.hairline,
        ..Style::default()
    };
    let mut halo_share = 1.0_f32;
    match settings.mode {
        0 => {
            style.fill = 0.90;
            style.rim = 0.85;
            style.halo = 0.55;
        }
        1 => {
            style.fill = 1.00;
            style.rim = 0.30;
            style.halo = 0.90;
            halo_share = 1.45;
        }
        2 => {
            style.fill = 0.95;
            style.rim = 0.45;
            style.halo = 0.60;
            style.banded = true;
            style.star_density = 1.0;
        }
        3 => {
            style.fill = 0.95;
            style.rim = 0.55;
            style.halo = 0.50;
            style.banded = true;
            style.hue_spread = 1.0;
        }
        4 => {
            style.fill = 0.45;
            style.rim = 1.00;
            style.halo = 0.85;
            style.inner_glow = clamp(settings.thickness, 4.0, 40.0) * 0.30;
        }
        5 => {
            style.fill = 0.75;
            style.rim = 0.55;
            style.halo = 0.45;
            style.banded = true;
        }
        6 => {
            style.fill = 0.90;
            style.rim = 0.65;
            style.halo = 0.50;
            style.glass = clamp01(settings.glass_tint);
            style.ghost = clamp(settings.refraction, 0.0, 3.0) * 2.0 + 1.0;
            style.banded = true;
        }
        7 => {
            style.fill = 0.0;
            style.rim = 1.00;
            style.halo = 0.35;
            style.rim_only = true;
        }
        8 => {
            style.fill = 0.18;
            style.rim = 1.00;
            style.halo = 0.50;
            style.inner_glow = clamp(settings.thickness, 4.0, 40.0) * 0.55;
            style.double_rim = true;
        }
        9 => {
            style.fill = 0.95;
            style.rim = 0.70;
            style.halo = 0.40;
            style.banded = true;
            style.cel = clamp01(settings.cel_opacity);
        }
        _ => {
            style.fill = 0.85;
            style.rim = 0.80;
            style.halo = 0.70;
        }
    }
    let glow = clamp(settings.glow_radius, 1.0, 60.0);
    style.halo_width = glow * halo_share + clamp(settings.blur, 0.0, 40.0) * 0.35;
     
     
    style.rim *= 1.0 - 0.45 * clamp01(clamp(settings.blur, 0.0, 40.0) / 40.0);
    if !settings.outline {
        style.rim = 0.0;
    }
    style
}

 
 
 

 
 
 
pub(crate) fn beat(
    settings: &Settings,
    clock_seconds: f64,
    playing: bool,
    playhead_seconds: f64,
) -> f32 {
    if !playing && !settings.idle_beats {
        return 1.0;
    }
    let phase = if playing {
        playhead_seconds
    } else {
        clock_seconds
    };
    let wave = 0.5 + 0.5 * (phase as f32 * 4.0 * std::f32::consts::PI).sin();
    let sensitivity = clamp(settings.audio_sensitivity, 0.0, 2.0);
    let depth = clamp(settings.reactivity, 0.0, 1.5) * (0.35 + 0.325 * sensitivity);
    clamp(1.0 + (wave - 0.5) * 2.0 * depth, 0.0, 2.0)
}

 
 
 
pub(crate) fn beat_ages(time_seconds: f32) -> [f32; 4] {
    const PERIOD: f32 = 0.5;
    let phase = time_seconds.rem_euclid(PERIOD);
    [
        phase,
        phase + PERIOD,
        phase + PERIOD * 2.0,
        phase + PERIOD * 3.0,
    ]
}

 
 
 

 
 
 
pub(crate) fn blur_sigma(settings: &Settings, px: f32) -> f32 {
    let mut radius = clamp(settings.glow_radius, 1.0, 60.0);
    if settings.mode == 1 {
        radius = radius.min(8.0);
    }
    let mut sigma = (radius * px / 1.6).max(0.5);
    match settings.mode {
        7 | 9 => sigma = (settings.outline_width * px).max(0.75),
        6 => sigma = (settings.thickness * px * 0.6).max(1.0),
        8 => sigma = (settings.glow_radius * px).max(2.0),
        10 => sigma = (settings.glow_radius * px * 0.8).max(1.0),
        _ => {}
    }
    sigma.min(16.0)
}

 
 
pub(crate) fn glow_divisor(sigma: f32) -> i32 {
    if sigma <= 6.0 {
        2
    } else {
        4
    }
}

 
 
pub(crate) fn wants_scene(mode: usize) -> bool {
     
    mode == 1 || mode == 6
}

 
 
pub(crate) fn scene_divisor(settings: &Settings) -> i32 {
    if settings.mode == 6 {
        return 2;
    }
    let blur_px = clamp(settings.blur, 0.0, 40.0);
    if blur_px <= 8.0 {
        2
    } else {
        4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

     
    fn shipped() -> Settings {
        Settings::default()
    }

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() <= 1e-4
    }

    #[test]
    fn the_eleven_looks_are_the_sources_and_each_keeps_its_index() {
        assert_eq!(MODE_COUNT, 11);
        assert_eq!(MODE_NAMES[0], "Glow");
        assert_eq!(MODE_NAMES[6], "Liquid Glass");
        assert_eq!(MODE_NAMES[10], "Pulse");
        for (i, name) in MODE_NAMES.iter().enumerate() {
            for other in MODE_NAMES.iter().skip(i + 1) {
                assert_ne!(name, other, "no two looks share a name");
            }
        }
    }

    #[test]
    fn the_defaults_are_the_sources_own() {
        let s = shipped();
        assert_eq!(s.mode, 0);
        assert!(near(s.glow_radius, 12.0));
        assert!(near(s.intensity, 1.0));
        assert!(near(s.blur, 10.0));
        assert!(near(s.tint_amount, 0.35));
        assert!(near(s.speed, 1.0));
        assert!(near(s.outline_width, 2.5));
        assert!(near(s.thickness, 14.0));
        assert!(near(s.refraction, 1.0));
        assert!(near(s.dispersion, 0.25));
        assert!(near(s.frost, 0.0));
        assert!(near(s.glass_tint, 0.25));
        assert!(near(s.lines, 6.0));
        assert!(near(s.cel_opacity, 0.9));
        assert!(near(s.reactivity, 0.6));
        assert!(s.idle_beats);
        assert!(near(s.audio_sensitivity, 1.0));
         
         
         
        assert!(near(s.fill_opacity, 1.0));
    }

    #[test]
    fn every_row_moves_the_number_it_claims_to() {
        let glow = compute_style(&shipped());
        assert!(glow.fill > 0.5 && glow.rim > 0.0 && glow.halo > 0.0);
        assert!(near(glow.halo_width, 12.0 + 10.0 * 0.35));

        let mut wide = shipped();
        wide.glow_radius = 40.0;
        assert!(compute_style(&wide).halo_width > glow.halo_width + 20.0);
        let mut beyond = shipped();
        beyond.glow_radius = 999.0;
        let mut at_sixty = shipped();
        at_sixty.glow_radius = 60.0;
        assert!(near(
            compute_style(&beyond).halo_width,
            compute_style(&at_sixty).halo_width
        ));

        let mut soft = shipped();
        soft.blur = 40.0;
        assert!(compute_style(&soft).rim < glow.rim);
        assert!(compute_style(&soft).halo_width > glow.halo_width);

        let mut no_outline = shipped();
        no_outline.outline = false;
        assert!(near(compute_style(&no_outline).rim, 0.0));

        let mut thick = shipped();
        thick.outline_width = 9.0;
        assert!(near(compute_style(&thick).rim_width, 9.0));
        thick.outline_width = 99.0;
        assert!(near(compute_style(&thick).rim_width, 12.0));
        thick.outline_width = 0.0;
        assert!(near(compute_style(&thick).rim_width, 0.5));

        let mut bands = shipped();
        bands.lines = 13.4;
        assert_eq!(compute_style(&bands).lines, 13);
        bands.lines = 99.0;
        assert_eq!(compute_style(&bands).lines, 16);
        bands.lines = 0.0;
        assert_eq!(compute_style(&bands).lines, 2);

        let mut splitting = shipped();
        splitting.dispersion = 1.0;
        assert!(near(compute_style(&splitting).dispersion, 4.0));

        let mut frosted = shipped();
        frosted.frost = 20.0;
        assert!(near(compute_style(&frosted).frost, 1.0));
        frosted.frost = 200.0;
        assert!(near(compute_style(&frosted).frost, 1.0));

        let mut tinted = shipped();
        tinted.tint_amount = 1.0;
        assert!(near(compute_style(&tinted).tint_mix, 1.0));
        tinted.tint_amount = -1.0;
        assert!(near(compute_style(&tinted).tint_mix, 0.0));
    }

    #[test]
    fn each_look_has_its_own_shape() {
        let mut s = shipped();
        let default_rim = compute_style(&s).rim_width;
        s.mode = 7;  
        let outline = compute_style(&s);
        assert!(near(outline.fill, 0.0) && outline.rim_only);
        s.mode = 3;  
        let prism = compute_style(&s);
        assert!(prism.banded && near(prism.hue_spread, 1.0));
        s.mode = 2;  
        assert!(near(compute_style(&s).star_density, 1.0));
        s.mode = 9;  
        assert!(near(compute_style(&s).cel, 0.9));
        s.mode = 6;  
        let glass = compute_style(&s);
        assert!(near(glass.glass, 0.25) && glass.ghost > 0.0);
        s.refraction = 3.0;
        assert!(compute_style(&s).ghost > glass.ghost);
        s.mode = 8;  
        assert!(compute_style(&s).double_rim);
        s.mode = 4;  
        assert!(compute_style(&s).inner_glow > 0.0);

         
         
        let mut strong = shipped();
        strong.intensity = 3.0;
        assert!(near(compute_style(&strong).rim_width, default_rim));
    }

    #[test]
    fn the_blur_radius_follows_the_look() {
        let mut s = shipped();
        let px = 1.0;
        let glow = blur_sigma(&s, px);
        assert!(near(glow, 12.0 / 1.6));
        s.mode = 7;  
        assert!(near(blur_sigma(&s, px), 2.5));
        s.mode = 9;  
        assert!(near(blur_sigma(&s, px), 2.5));
        s.mode = 6;  
        assert!(near(blur_sigma(&s, px), 14.0 * 0.6));
        s.mode = 1;  
        s.glow_radius = 40.0;
        assert!(near(blur_sigma(&s, px), 8.0 / 1.6));
        s.mode = 8;  
        assert!(near(blur_sigma(&s, px), 16.0));
         
        assert!(near(blur_sigma(&s, 4.0), 16.0));
         
        assert_eq!(glow_divisor(blur_sigma(&shipped(), px)), 4);
        assert_eq!(glow_divisor(2.0), 2);
        assert_eq!(glow_divisor(20.0), 4);
    }

    #[test]
    fn the_two_see_through_looks_ask_for_the_frame() {
        assert!(wants_scene(1), "Blur paints the scene it blurs");
        assert!(wants_scene(6), "Liquid Glass refracts the scene");
        for mode in [0, 2, 3, 4, 5, 7, 8, 9, 10] {
            assert!(!wants_scene(mode));
        }
         
        let mut s = shipped();
        s.mode = 6;
        assert_eq!(scene_divisor(&s), 2);
        s.mode = 0;
        s.blur = 40.0;
        assert_eq!(scene_divisor(&s), 4);
    }

    #[test]
    fn the_beat_only_pulses_when_it_should() {
        let mut s = shipped();
        s.idle_beats = false;
        assert!(near(beat(&s, 0.3, false, 0.0), 1.0));
        s.idle_beats = true;
        let value = beat(&s, 0.1, false, 0.0);
        assert!((0.0..=2.0).contains(&value));
         
        assert_ne!(beat(&s, 0.0, true, 0.125), beat(&s, 0.9, false, 0.0));
         
        let mut deaf = s;
        deaf.audio_sensitivity = 0.0;
        let quiet = (beat(&deaf, 0.125, false, 0.0) - 1.0).abs();
        let loud = (beat(&s, 0.125, false, 0.0) - 1.0).abs();
        assert!(quiet < loud);
    }

    #[test]
    fn the_beat_ages_are_a_half_second_apart() {
        let ages = beat_ages(0.2);
        assert!(near(ages[0], 0.2));
        assert!(near(ages[1], 0.7));
        assert!(near(ages[3], 1.7));
         
        assert!(near(beat_ages(1.2)[0], 0.2));
    }

     
     
    #[test]
    fn the_two_kinds_switch_back_and_forth() {
        let mut module = crate::module::render::chams::ChamsModule::new();
        let data = &mut module.module;
        assert!(data.get_setting("Outline Glow").is_some());
        assert!(data.get_setting("Glow Theme").is_none());
        let scanline = 2;
        let glass = FIRST_STYLE + 6;
        set_style(data, scanline);
        assert!(is_warped(data));
        assert_eq!(selected_mode(data), Some(5));
        assert_eq!(classic_style(style_index(data) as i32), scanline as i32);
        if let Some(ModuleSetting::Toggle { value, .. }) = data.get_setting_mut("Outline Glow") {
            *value = false;
        }
        assert!(!is_warped(data), "the toggle disables the glow pass");
        if let Some(ModuleSetting::Toggle { value, .. }) = data.get_setting_mut("Outline Glow") {
            *value = true;
        }
        assert!(is_warped(data));
        set_style(data, glass);
        assert!(
            is_warped(data),
            "the Warped look paints as soon as it is chosen"
        );
        assert_eq!(selected_mode(data), Some(6));
        assert_eq!(Settings::read(data).mode_name(), "Liquid Glass");
         
        set_style(data, scanline);
        assert!(is_warped(data));
        assert_eq!(Settings::read(data).mode, 5);
        assert_eq!(classic_style(style_index(data) as i32), scanline as i32);
         
        for style in (0..=LAST_STYLE).rev() {
            set_style(data, style);
            assert_eq!(style_index(data), style);
            assert!(is_warped(data));
            assert_eq!(
                classic_style(style_index(data) as i32),
                if style < FIRST_STYLE { style as i32 } else { 0 }
            );
        }
    }

     
     
    fn set_style(data: &mut ModuleData, style: usize) {
        match data.get_setting_mut("Style") {
            Some(ModuleSetting::Choice { value, .. }) => *value = style,
            other => panic!("Style is not a choice: {other:?}"),
        }
    }

    #[test]
    fn the_looks_are_entries_of_the_one_style_dropdown() {
        let mut module = crate::module::render::chams::ChamsModule::new();
        let styles = crate::module::render::chams::STYLES;
         
        for style in 0..FIRST_STYLE {
            set_style(&mut module.module, style);
            assert!(
                is_warped(&module.module),
                "style {style} receives outline glow"
            );
            assert_eq!(selected_mode(&module.module), Some([4, 9, 5, 10, 0][style]));
            assert_eq!(classic_style(style as i32), style as i32);
        }
        assert_eq!(styles.len(), LAST_STYLE + 1);
        for (mode, name) in MODE_NAMES.iter().enumerate() {
            let style = FIRST_STYLE + mode;
            set_style(&mut module.module, style);
            assert_eq!(styles[style], *name, "{name} sits at {style}");
            assert_eq!(selected_mode(&module.module), Some(mode));
            assert!(is_warped(&module.module));
            assert_eq!(Settings::read(&module.module).mode, mode);
        }
         
        set_style(&mut module.module, FIRST_STYLE + 6);
        assert_eq!(Settings::read(&module.module).mode_name(), "Liquid Glass");
         
         
        assert_eq!(classic_style(9), 0);
         
        for name in [
            "Glow Radius",
            "Intensity",
            "Fill Opacity",
            "Blur",
            "Tint Amount",
            "Speed",
            "Outline Width",
            "Thickness",
            "Refraction",
            "Dispersion",
            "Frost",
            "Glass Tint",
            "Lines",
            "Cel Opacity",
            "Reactivity",
            "Audio Sensitivity",
        ] {
            assert!(
                module.module.get_setting(name).is_some(),
                "{name} is missing from Chams"
            );
        }
        for name in ["Outline", "Hairline", "Idle Beats"] {
            assert!(
                module.module.get_setting(name).is_some(),
                "{name} is missing from Chams"
            );
        }
         
        set_style(&mut module.module, 0);
        let read = Settings::read(&module.module);
        assert_eq!(read.mode, 0);
        assert!(near(read.glow_radius, 12.0));
    }
}
