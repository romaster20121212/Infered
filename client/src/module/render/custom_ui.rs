 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::{anim, theme};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use egui::{Color32, Context, Id, Vec2};
use std::sync::atomic::{AtomicBool, Ordering};

 
pub const CRIMSON: [f32; 4] = [0.63, 0.07, 0.09, 1.0];
 
pub const INK: [f32; 4] = [0.045, 0.025, 0.03, 1.0];
 
pub const BONE: [f32; 4] = [0.95, 0.91, 0.91, 1.0];
 
pub const ECHO: [f32; 4] = [1.0, 0.36, 0.42, 1.0];

 
 
 
pub static EDITOR_OPEN: AtomicBool = AtomicBool::new(false);

 
fn rgba(value: [f32; 4], alpha: f32) -> Color32 {
    let channel = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    let a = (value[3].clamp(0.0, 1.0) * alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color32::from_rgba_unmultiplied(channel(value[0]), channel(value[1]), channel(value[2]), a)
}

fn toggle(data: &ModuleData, name: &str, default: bool) -> bool {
    data.get_setting(name)
        .and_then(ModuleSetting::get_toggle_value)
        .unwrap_or(default)
}

fn slider(data: &ModuleData, name: &str, default: f32) -> f32 {
    data.get_setting(name)
        .and_then(ModuleSetting::get_slider_value)
        .filter(|value| value.is_finite())
        .unwrap_or(default)
}

fn choice(data: &ModuleData, name: &str, default: usize) -> usize {
    match data.get_setting(name) {
        Some(ModuleSetting::Choice { value, .. }) => *value,
        _ => default,
    }
}

fn color(data: &ModuleData, name: &str, default: [f32; 4]) -> [f32; 4] {
    match data.get_setting(name) {
        Some(ModuleSetting::Color { value, .. }) => *value,
        _ => default,
    }
}

 
#[derive(Clone, Copy, Debug)]
pub struct UiStyle {
    pub accent: Color32,
    pub background: Color32,
    pub text: Color32,
    pub border_alpha: f32,
    pub shadow_alpha: f32,
    pub shadow_offset: f32,
    pub glow: bool,
    pub glow_strength: f32,
    pub glow_pulse: bool,
    pub pulse_speed: f32,
    pub radius: f32,
    pub padding: f32,
    pub row_height: f32,
    pub text_size: f32,
    pub scale: f32,
    pub border_width: f32,
    pub lowercase: bool,
     
     
    pub follow_theme: bool,
     
    pub panel_gradient: bool,
    pub gradient_strength: f32,
     
    pub comets: bool,
     
    pub comet_speed: f32,
     
    pub comet_tail: f32,
     
    pub text_shadow: bool,
     
    pub animation: usize,
     
    pub animation_speed: f32,
     
    pub reveal_distance: f32,
     
    pub stagger: f32,
}

impl Default for UiStyle {
    fn default() -> Self {
        Self {
             
            accent: theme::accent(),
            background: theme::with_alpha(theme::base(), 0.84),
            text: theme::TEXT,
            border_alpha: 0.42,
            shadow_alpha: 0.48,
            shadow_offset: 2.0,
            glow: true,
            glow_strength: 0.34,
            glow_pulse: true,
            pulse_speed: 0.25,
            radius: theme::RADIUS,
            padding: 8.0,
            row_height: 20.0,
            text_size: 13.5,
            scale: 1.0,
            border_width: 1.0,
            lowercase: false,
            follow_theme: true,
            panel_gradient: true,
            gradient_strength: 0.12,
            comets: true,
            comet_speed: 0.16,
            comet_tail: 0.3,
            text_shadow: false,
            animation: 0,
            animation_speed: 1.0,
            reveal_distance: 12.0,
            stagger: 0.06,
        }
    }
}

 
 
 
pub fn is_enabled() -> bool {
    let Some(client) = crate::state::try_client() else {
        return false;
    };
    let Some(handle) = client.modules.get(ModuleId::CustomUi) else {
        return false;
    };
    handle
        .lock()
        .map(|module| module.get_module_data().enabled)
        .unwrap_or(false)
}

 
 
 
pub fn pause_menu_active() -> bool {
    let Some(client) = crate::state::try_client() else {
        return false;
    };
    let Some(handle) = client.modules.get(ModuleId::CustomUi) else {
        return false;
    };
    let Ok(module) = handle.lock() else {
        return false;
    };
    pause_menu_on(module.get_module_data())
}

 
 
fn pause_menu_on(data: &ModuleData) -> bool {
    data.enabled && toggle(data, "Pause Menu", true)
}

 
 
 
pub fn hotbar_active() -> bool {
    let Some(client) = crate::state::try_client() else {
        return false;
    };
    let Some(handle) = client.modules.get(ModuleId::CustomUi) else {
        return false;
    };
    let Ok(module) = handle.lock() else {
        return false;
    };
    hotbar_on(module.get_module_data())
}

 
 
fn hotbar_on(data: &ModuleData) -> bool {
    data.enabled && toggle(data, "Hotbar", true)
}

 
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RevealPose {
    pub alpha: f32,
    pub offset: Vec2,
}

impl RevealPose {
    pub fn is_visible(self) -> bool {
        self.alpha > 0.001
    }
}

impl UiStyle {
     
     
     
     
    pub fn read() -> Self {
        let fallback = Self::default();
        let Some(client) = crate::state::try_client() else {
            return fallback;
        };
        let Some(handle) = client.modules.get(ModuleId::CustomUi) else {
            return fallback;
        };
        let Ok(module) = handle.lock() else {
            return fallback;
        };
        Self::from_data(module.get_module_data())
    }

     
    pub fn reveal_seconds(&self) -> f32 {
        0.42 / self.animation_speed.clamp(0.25, 3.0)
    }

     
     
    pub fn reveal_at(&self, progress: f32, index: usize, count: usize) -> RevealPose {
        let progress = progress.clamp(0.0, 1.0);
        let per_widget = (self.stagger / self.reveal_seconds()).max(0.0);
        let total = per_widget * count.saturating_sub(1) as f32;
        let k = (progress * (1.0 + total) - index as f32 * per_widget).clamp(0.0, 1.0);
        let alpha = k;
        let eased = anim::Easing::OutBack.apply(k).clamp(0.0, 1.0);
        let distance = (1.0 - eased) * self.reveal_distance;
        let offset = match self.animation {
            1 => Vec2::new(0.0, -distance),
            2 => Vec2::new(distance, 0.0),
            3 => Vec2::ZERO,
            _ => Vec2::new(0.0, distance),
        };
        RevealPose { alpha, offset }
    }

     
     
    fn from_data(data: &ModuleData) -> Self {
        let follow_theme = toggle(data, "Follow Theme", true);
        let accent = if follow_theme {
            theme::accent()
        } else {
            rgba(color(data, "Accent", CRIMSON), 1.0)
        };
        let background = if follow_theme {
            theme::base()
        } else {
            rgba(color(data, "Background", INK), 1.0)
        };
        let text = if follow_theme {
            theme::TEXT
        } else {
            rgba(color(data, "Text", BONE), 1.0)
        };
        let opacity = slider(data, "Panel Opacity", 0.84).clamp(0.0, 1.0);
        Self {
            accent,
            background: theme::with_alpha(background, opacity),
            text,
            border_alpha: slider(data, "Border Opacity", 0.42).clamp(0.0, 1.0),
            shadow_alpha: slider(data, "Shadow Opacity", 0.48).clamp(0.0, 1.0),
            shadow_offset: slider(data, "Shadow Offset", 2.0).clamp(0.0, 8.0),
            glow: toggle(data, "Glow", true),
            glow_strength: slider(data, "Glow Strength", 0.34).clamp(0.0, 1.0),
            glow_pulse: toggle(data, "Glow Pulse", true),
            pulse_speed: slider(data, "Pulse Speed", 0.25).clamp(0.05, 1.5),
            radius: slider(data, "Corner Radius", theme::RADIUS).clamp(0.0, 18.0),
            padding: slider(data, "Padding", 8.0).clamp(2.0, 18.0),
            row_height: slider(data, "Row Height", 20.0).clamp(14.0, 32.0),
            text_size: slider(data, "Text Size", 13.5).clamp(8.0, 22.0),
            scale: slider(data, "Widget Scale", 1.0).clamp(0.5, 2.0),
            border_width: slider(data, "Border Width", 1.0).clamp(0.0, 3.0),
            lowercase: toggle(data, "Lowercase", false),
            follow_theme,
            panel_gradient: toggle(data, "Panel Gradient", true),
            gradient_strength: slider(data, "Gradient Strength", 0.12).clamp(0.0, 0.5),
            comets: toggle(data, "Comets", true),
            comet_speed: slider(data, "Comet Speed", 0.16).clamp(0.0, 0.8),
            comet_tail: slider(data, "Comet Tail", 0.3).clamp(0.05, 0.7),
            text_shadow: toggle(data, "Text Shadow", false),
            animation: choice(data, "Animation", 0),
            animation_speed: slider(data, "Animation Speed", 1.0).clamp(0.25, 3.0),
            reveal_distance: slider(data, "Reveal Distance", 12.0).clamp(0.0, 40.0),
            stagger: slider(data, "Stagger", 0.06).clamp(0.0, 0.2),
        }
    }

     
     
     
     
     
     
     
    pub fn hud_style() -> Self {
        if is_enabled() {
            Self::read()
        } else {
            Self::default()
        }
    }

     
    pub fn reveal_progress(ctx: &Context) -> f32 {
        let style = Self::read();
        anim::toggle(
            ctx,
            Id::new("custom_ui_reveal"),
            is_enabled(),
            style.reveal_seconds(),
            anim::Easing::Out,
        )
    }

     
     
     
     
     
    pub fn reveal_pose(ctx: &Context, index: usize, count: usize) -> RevealPose {
         
         
        let progress = Self::reveal_progress(ctx);
        if !is_enabled() {
            return RevealPose {
                alpha: 1.0,
                offset: Vec2::ZERO,
            };
        }
        Self::read().reveal_at(progress, index, count)
    }

     
    pub fn label(&self, text: &str) -> String {
        if self.lowercase {
            text.to_lowercase()
        } else {
            text.to_string()
        }
    }

     
     
    pub fn line_height(&self) -> f32 {
        self.row_height.max(self.text_size + 4.0) * self.scale
    }

     
    pub fn rounding(&self, rect: egui::Rect) -> egui::Rounding {
        egui::Rounding::same(self.radius.min(rect.height() * 0.5))
    }
}

 
 
fn palette_accent(data: &ModuleData) -> Color32 {
    if toggle(data, "Follow Theme", false) {
        theme::accent()
    } else {
        rgba(color(data, "Accent", ECHO), 1.0)
    }
}

 
fn palette_background(data: &ModuleData) -> Color32 {
    rgba(color(data, "Background", INK), 1.0)
}

 
#[derive(Clone, Copy, Debug)]
pub struct HotbarStyle {
     
    pub scale: f32,
     
    pub slide_speed: f32,
    pub glow: bool,
     
    pub numbers: bool,
     
    pub item_name: bool,
    pub accent: Color32,
    pub background: Color32,
    pub radius: f32,
}

impl Default for HotbarStyle {
    fn default() -> Self {
        Self {
            scale: 1.0,
            slide_speed: 1.15,
            glow: true,
            numbers: true,
            item_name: true,
            accent: rgba(ECHO, 1.0),
            background: rgba(INK, 1.0),
            radius: 6.0,
        }
    }
}

impl HotbarStyle {
    pub fn read() -> Self {
        let fallback = Self::default();
        let Some(client) = crate::state::try_client() else {
            return fallback;
        };
        let Some(handle) = client.modules.get(ModuleId::CustomUi) else {
            return fallback;
        };
        let Ok(module) = handle.lock() else {
            return fallback;
        };
        Self::from_data(module.get_module_data())
    }

    fn from_data(data: &ModuleData) -> Self {
        Self {
            scale: slider(data, "Hotbar Scale", 1.0).clamp(0.7, 1.6),
            slide_speed: slider(data, "Slide Speed", 1.15).clamp(0.4, 2.5),
            glow: toggle(data, "Hotbar Glow", true),
            numbers: toggle(data, "Hotbar Numbers", true),
            item_name: toggle(data, "Item Name", true),
            accent: palette_accent(data),
            background: palette_background(data),
            radius: slider(data, "Corner Radius", 7.0).clamp(0.0, 18.0),
        }
    }

     
     
     
    pub fn slide_spring(&self) -> anim::SpringCfg {
        let speed = self.slide_speed.clamp(0.4, 2.5);
        anim::SpringCfg {
            stiffness: 260.0 * speed * speed,
            damping: 28.0 * speed.sqrt(),
        }
    }
}

 
#[derive(Clone, Copy, Debug)]
pub struct PauseStyle {
     
    pub animations: bool,
    pub speed: f32,
     
    pub dim: f32,
    pub embers: bool,
    pub ember_count: usize,
    pub accent: Color32,
    pub background: Color32,
    pub radius: f32,
    pub glow: bool,
}

impl Default for PauseStyle {
    fn default() -> Self {
        Self {
            animations: true,
            speed: 1.0,
            dim: 0.66,
            embers: true,
            ember_count: 28,
            accent: rgba(ECHO, 1.0),
            background: rgba(INK, 1.0),
            radius: 9.0,
            glow: true,
        }
    }
}

impl PauseStyle {
    pub fn read() -> Self {
        let fallback = Self::default();
        let Some(client) = crate::state::try_client() else {
            return fallback;
        };
        let Some(handle) = client.modules.get(ModuleId::CustomUi) else {
            return fallback;
        };
        let Ok(module) = handle.lock() else {
            return fallback;
        };
        Self::from_data(module.get_module_data())
    }

    fn from_data(data: &ModuleData) -> Self {
        Self {
            animations: toggle(data, "Pause Animations", true),
            speed: slider(data, "Pause Speed", 1.0).clamp(0.4, 2.5),
            dim: slider(data, "Backdrop Dim", 0.66).clamp(0.0, 0.95),
            embers: toggle(data, "Embers", true),
            ember_count: slider(data, "Ember Count", 28.0).clamp(0.0, 80.0) as usize,
            accent: palette_accent(data),
            background: palette_background(data),
            radius: slider(data, "Corner Radius", 7.0).clamp(0.0, 18.0),
            glow: toggle(data, "Glow", true),
        }
    }
}

 
#[derive(Clone, Copy, Debug)]
pub struct EditorStyle {
    pub snap_to_grid: bool,
    pub grid_size: f32,
    pub snap_distance: f32,
    pub dim: f32,
    pub card_opacity: f32,
    pub show_labels: bool,
    pub show_sizes: bool,
    pub show_hint: bool,
    pub show_guides: bool,
    pub lock_widgets: bool,
    pub highlight_hovered: bool,
    pub hover_lift: bool,
    pub animate: bool,
    pub snap_to_edges: bool,
}

impl Default for EditorStyle {
    fn default() -> Self {
        Self {
            snap_to_grid: true,
            grid_size: 8.0,
            snap_distance: 9.0,
             
             
            dim: 0.62,
            card_opacity: 0.58,
            show_labels: true,
            show_sizes: true,
            show_hint: true,
            show_guides: true,
            lock_widgets: false,
            highlight_hovered: true,
            hover_lift: true,
            animate: true,
            snap_to_edges: true,
        }
    }
}

impl EditorStyle {
    pub fn read() -> Self {
        let fallback = Self::default();
        let Some(client) = crate::state::try_client() else {
            return fallback;
        };
        let Some(handle) = client.modules.get(ModuleId::CustomUi) else {
            return fallback;
        };
        let Ok(module) = handle.lock() else {
            return fallback;
        };
        Self::from_data(module.get_module_data())
    }

    fn from_data(data: &ModuleData) -> Self {
        Self {
            snap_to_grid: toggle(data, "Snap To Grid", true),
            grid_size: slider(data, "Grid Size", 8.0).clamp(2.0, 32.0),
            snap_distance: slider(data, "Snap Distance", 9.0).clamp(2.0, 20.0),
            dim: slider(data, "Editor Dim", 0.62).clamp(0.0, 0.85),
            card_opacity: slider(data, "Card Opacity", 0.58).clamp(0.0, 1.0),
            show_labels: toggle(data, "Show Labels", true),
            show_sizes: toggle(data, "Show Sizes", true),
            show_hint: toggle(data, "Show Hint", true),
            show_guides: toggle(data, "Show Guides", true),
            lock_widgets: toggle(data, "Lock Widgets", false),
            highlight_hovered: toggle(data, "Highlight Hovered", true),
            hover_lift: toggle(data, "Hover Lift", true),
            animate: toggle(data, "Animate Editor", true),
            snap_to_edges: toggle(data, "Snap To Edges", true),
        }
    }
}

 
 
 
 
 
 
pub fn set_editing(open: bool) {
    EDITOR_OPEN.store(open, Ordering::Relaxed);
    let Some(client) = crate::state::try_client() else {
        return;
    };
    let Some(handle) = client.modules.get(ModuleId::CustomUi) else {
        return;
    };
    let Ok(mut module) = handle.lock() else {
        return;
    };
    if let Some(setting) = module.get_module_data_mut().get_setting_mut("Edit Layout") {
        setting.set_toggle_value(open);
    }
}

 
 
 
 
 
 
 
pub fn editor_open() -> bool {
    let current = EDITOR_OPEN.load(Ordering::Relaxed);
    let Some(client) = crate::state::try_client() else {
        return current;
    };
    let Some(handle) = client.modules.get(ModuleId::CustomUi) else {
        return current;
    };
    let Ok(module) = handle.lock() else {
        return current;
    };
    let data = module.get_module_data();
    if !data.enabled {
        return current;
    }
    let wanted = toggle(data, "Edit Layout", false);
    if wanted != current {
        EDITOR_OPEN.store(wanted, Ordering::Relaxed);
        wanted
    } else {
        current
    }
}

 
 
 
 
 
 
 
 
 
 
#[derive(Debug)]
pub struct CustomUiModule {
    pub module: ModuleData,
}

impl CustomUiModule {
    pub fn new() -> Self {
        let setting = |name: &str| name.to_string();
        Self {
            module: ModuleData {
                id: ModuleId::CustomUi,
                description: "Replaces the pause screen and draws the custom hotbar".to_string(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                 
                 
                enabled: false,
                settings: vec![
                    ModuleSetting::Toggle {
                        name: setting("Pause Menu"),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: setting("Pause Animations"),
                        value: true,
                    },
                    ModuleSetting::Slider {
                        name: setting("Pause Speed"),
                        value: 1.0,
                        min: 0.4,
                        max: 2.5,
                    },
                    ModuleSetting::Slider {
                        name: setting("Backdrop Dim"),
                        value: 0.66,
                        min: 0.0,
                        max: 0.95,
                    },
                    ModuleSetting::Toggle {
                        name: setting("Embers"),
                        value: true,
                    },
                    ModuleSetting::Slider {
                        name: setting("Ember Count"),
                        value: 28.0,
                        min: 0.0,
                        max: 80.0,
                    },
                    ModuleSetting::Toggle {
                        name: setting("Hotbar"),
                        value: true,
                    },
                    ModuleSetting::Slider {
                        name: setting("Hotbar Scale"),
                        value: 1.0,
                        min: 0.7,
                        max: 1.6,
                    },
                    ModuleSetting::Slider {
                        name: setting("Slide Speed"),
                        value: 1.15,
                        min: 0.4,
                        max: 2.5,
                    },
                    ModuleSetting::Toggle {
                        name: setting("Hotbar Glow"),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: setting("Hotbar Numbers"),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: setting("Item Name"),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: setting("Follow Theme"),
                        value: false,
                    },
                    ModuleSetting::Color {
                        name: setting("Accent"),
                        value: ECHO,
                    },
                    ModuleSetting::Color {
                        name: setting("Background"),
                        value: INK,
                    },
                    ModuleSetting::Slider {
                        name: setting("Corner Radius"),
                        value: 7.0,
                        min: 0.0,
                        max: 18.0,
                    },
                    ModuleSetting::Toggle {
                        name: setting("Edit Layout"),
                        value: false,
                    },
                ],
            },
        }
    }
}

impl Module for CustomUiModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        EDITOR_OPEN.store(false, Ordering::Relaxed);
         
         
        crate::graphic::pause_menu::stand_down();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}

#[cfg(test)]
#[path = "custom_ui_tests.rs"]
mod tests;
