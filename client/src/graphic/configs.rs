 
 
 
 
 
 
 
 
 
 
 
 

use crate::config;
use crate::graphic::ambience;
use crate::graphic::anim::{self, Easing};
use crate::graphic::friends;
use crate::graphic::settings;
use crate::graphic::sidebar;
use crate::graphic::theme;
use crate::graphic::transparency::{self, Layer};
use egui::{
    Align, Align2, Context, Frame, Id, Layout, Margin, Order, Rect, RichText, Rounding, Sense,
    Stroke, Ui, Vec2,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};

 
const WIDTH: f32 = 340.0;
 
const RISE: f32 = 14.0;
 
const OPEN_TIME: f32 = 0.26;
 
const LIST_H: f32 = 196.0;
 
const ROW_H: f32 = 34.0;
 
const LOAD_W: f32 = 64.0;
 
const ROW_GAP: f32 = 6.0;

 
static OPEN: AtomicBool = AtomicBool::new(false);
 
static NAMES: LazyLock<Mutex<Vec<String>>> = LazyLock::new(|| Mutex::new(Vec::new()));
 
static LOADED: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));
 
static ERROR: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));

 
pub fn is_open() -> bool {
    OPEN.load(Ordering::Relaxed)
}

 
 
pub fn toggle_open() {
    let was_open = OPEN.fetch_xor(true, Ordering::Relaxed);
    if !was_open {
        settings::close();
        friends::close();
        refresh();
    }
}

 
pub fn close() {
    OPEN.store(false, Ordering::Relaxed);
}

 
 
pub fn refresh() {
    let names = config::named_configs();
    if let Ok(mut list) = NAMES.lock() {
        *list = names;
    }
}

 
 
#[cfg(test)]
pub(crate) fn window_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

 
 
pub fn sync_with_menu(gui_open: bool) {
    if !gui_open {
        close();
    }
}

 
 
pub fn progress(ctx: &Context, gui_progress: f32) -> f32 {
    let target = is_open() && gui_progress > 0.0;
    anim::toggle(
        ctx,
        Id::new("configs_window"),
        target,
        OPEN_TIME,
        Easing::Out,
    )
}

 
pub fn is_visible(progress: f32) -> bool {
    progress > 0.001
}

 
pub fn draw(ctx: &Context, p: f32) {
    if !is_visible(p) {
        return;
    }
     
    ctx.request_repaint();

    egui::Area::new(Id::new("clickgui_configs"))
        .order(Order::Foreground)
        .anchor(Align2::CENTER_CENTER, Vec2::new(0.0, RISE * (1.0 - p)))
        .show(ctx, |ui| {
            ui.set_opacity(p);
            let window = Frame::none()
                 
                 
                .fill(transparency::surface(0.93, Layer::Window))
                .stroke(Stroke::new(1.0_f32, theme::BORDER))
                .rounding(Rounding::same(theme::RADIUS))
                .shadow(ui.style().visuals.window_shadow)
                .inner_margin(Margin::same(14.0))
                .show(ui, |ui| {
                    ui.set_width(WIDTH);
                    ui.spacing_mut().item_spacing.y = 6.0;
                    header(ui, p);
                    ui.add_space(2.0);
                    body(ui);
                });
            transparency::edge(
                ui.painter(),
                window.response.rect,
                Rounding::same(theme::RADIUS),
                Layer::Window,
                0.9 * p,
            );
            ambience::panel_light(
                ui.painter(),
                window.response.rect,
                theme::RADIUS,
                theme::accent(),
                0.8 * p,
            );
        });
}

 
 
fn header(ui: &mut Ui, p: f32) {
    let mut title = None;
    let mut refresh_now = false;
    ui.horizontal(|ui| {
        let response = ui.label(
            RichText::new("Configs")
                .size(16.0)
                .strong()
                .color(theme::TEXT),
        );
        title = Some(response.rect);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if sidebar::pressable(ui, "Close", false, 64.0).clicked() {
                close();
            }
            refresh_now = sidebar::pressable(ui, "Refresh", false, 74.0).clicked();
        });
    });
    if refresh_now {
        refresh();
    }

    let Some(title) = title else {
        return;
    };
    let t = Easing::Out.apply(p).clamp(0.0, 1.0);
    if t <= 0.001 {
        return;
    }
    let painter = ui.painter();
    let line = Rect::from_min_size(
        egui::pos2(title.left(), title.bottom() + 2.0),
        egui::vec2(title.width().max(48.0) * t, 1.5),
    );
    painter.rect_filled(
        line,
        Rounding::ZERO,
        theme::with_alpha(theme::accent(), 0.72),
    );
    let head = line.right_center();
    painter.circle_filled(head, 4.5, theme::with_alpha(theme::accent(), 0.16));
    painter.circle_filled(head, 1.8, theme::accent());
}

 
fn body(ui: &mut Ui) {
    let names = NAMES.lock().map(|list| list.clone()).unwrap_or_default();
    section(ui, "Profiles");
    let name_id = Id::new("new_profile_name");
    let mut new_name = ui.data(|data| data.get_temp::<String>(name_id)).unwrap_or_default();
    let mut save_profile = false;
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut new_name).hint_text("New profile name").desired_width(220.0));
        save_profile = ui.button("Save").clicked();
    });
    if save_profile {
        match config::save_named(&new_name) {
            Ok(()) => {
                set_loaded(Some(new_name.trim().to_owned()));
                set_error(None);
                new_name.clear();
                refresh();
            }
            Err(error) => set_error(Some(error)),
        }
    }
    ui.data_mut(|data| data.insert_temp(name_id, new_name));

    if names.is_empty() {
        ui.label(
            RichText::new("No profiles yet — enter a name and save one above.")
                .size(11.5)
                .color(theme::TEXT_MUTED),
        );
    } else {
        let loaded = LOADED.lock().ok().and_then(|name| name.clone());
        let mut load = None;
        egui::ScrollArea::vertical()
            .id_salt("configs_list")
            .max_height(LIST_H)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for name in &names {
                    let is_loaded = loaded.as_deref() == Some(name.as_str());
                    if config_row(ui, name, is_loaded) {
                        load = Some(name.clone());
                    }
                }
            });
        if let Some(name) = load {
            match config::load_named(&name) {
                Ok(()) => {
                    set_loaded(Some(name));
                    set_error(None);
                }
                Err(error) => set_error(Some(error)),
            }
        }
    }

    ui.add_space(6.0);
    section(ui, "Keybinds");
    let registry = crate::state::client().modules.by_id();
    let mut ids: Vec<_> = registry.keys().copied().collect();
    ids.sort_by_key(|id| id.display_name());
    let mut changed = false;
    egui::ScrollArea::vertical().id_salt("profile_keybinds").max_height(170.0).show(ui, |ui| {
        for id in ids {
            let Some(arc) = registry.get(&id) else { continue; };
            let Ok(mut module) = arc.lock() else { continue; };
            let data = module.get_module_data_mut();
            let before = data.key_bind;
            ui.push_id(id, |ui| {
                ui.label(RichText::new(id.display_name()).size(11.5));
                crate::graphic::menu::keybind_row(ui, data, arc, &registry, 1.0);
            });
            changed |= before != data.key_bind;
        }
    });
    if changed {
        config::save();
    }

    if let Some(error) = ERROR.lock().ok().and_then(|error| error.clone()) {
        ui.add_space(2.0);
        ui.label(RichText::new(error).size(11.0).color(theme::DANGER));
    }
}

 
fn section(ui: &mut Ui, title: &str) {
    ui.label(
        RichText::new(title.to_uppercase())
            .size(10.0)
            .strong()
            .color(theme::accent()),
    );
}

 
 
 
fn config_row(ui: &mut Ui, name: &str, loaded: bool) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        let row_w = (ui.available_width() - LOAD_W - ROW_GAP).max(80.0);
        if name_row(ui, name, loaded, row_w).clicked() {
            clicked = true;
        }
        let label = if loaded { "Reload" } else { "Load" };
        if sidebar::pressable(ui, label, loaded, LOAD_W).clicked() {
            clicked = true;
        }
    });
    clicked
}

 
 
fn name_row(ui: &mut Ui, name: &str, loaded: bool, width: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, ROW_H), Sense::click());
    let hover = anim::toggle(
        ui.ctx(),
        Id::new("config_row_hover").with(name),
        response.hovered(),
        0.14,
        Easing::Out,
    );
    let painter = ui.painter();
    painter.rect(
        rect,
        Rounding::same(theme::RADIUS_INNER),
        theme::lerp_color(theme::SURFACE, theme::SURFACE_HOVER, hover),
        Stroke::new(
            1.0_f32,
            theme::lerp_color(
                theme::BORDER,
                theme::accent(),
                if loaded { 0.55 } else { 0.0 },
            ),
        ),
    );
    painter.text(
        egui::pos2(rect.left() + 10.0, rect.center().y),
        Align2::LEFT_CENTER,
        name,
        egui::FontId::proportional(12.0),
        if loaded { theme::accent() } else { theme::TEXT },
    );
    response
}

 
fn set_loaded(name: Option<String>) {
    if let Ok(mut loaded) = LOADED.lock() {
        *loaded = name;
    }
}

 
fn set_error(error: Option<String>) {
    if let Ok(mut slot) = ERROR.lock() {
        *slot = error;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

     
     
    #[test]
    fn opening_configs_closes_the_other_modals_and_the_menu_closes_it() {
         
         
        let _guard = settings::window_lock();
         
        close();
        settings::close();
        friends::close();

         
        settings::toggle_open();
        assert!(settings::is_open());
        friends::set_open(true);
        assert!(friends::is_open());
        toggle_open();
        assert!(is_open(), "Configs opens");
        assert!(!settings::is_open(), "opening Configs closes Settings");
        assert!(!friends::is_open(), "opening Configs closes Friends");

         
        settings::toggle_open();
        assert!(settings::is_open());
        assert!(!is_open(), "opening Settings closes Configs");
        friends::set_open(true);
        assert!(!is_open(), "opening Friends closes Configs");

        sync_with_menu(true);
        assert!(!is_open(), "a closed window stays closed");
        toggle_open();
        assert!(is_open(), "Configs reopens");
        sync_with_menu(true);
        assert!(is_open(), "an open menu leaves the window alone");
        sync_with_menu(false);
        assert!(!is_open(), "closing the menu closes Configs");
        friends::close();
    }

     
     
     
    #[test]
    #[ignore = "writes a preview into screenshots/"]
    fn configs_window_preview() {
        use crate::graphic::preview::{gather_textures, rasterize};
        use std::collections::HashMap;

        if let Ok(mut names) = NAMES.lock() {
            *names = ["Main PVP", "HvH", "Bridging", "Crystal PvP"]
                .into_iter()
                .map(str::to_owned)
                .collect();
        }
        set_loaded(Some("HvH".to_string()));

        let (width, height) = (760usize, 560usize);
        let screen = Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(width as f32, height as f32));
        let ctx = Context::default();
        let mut textures = HashMap::new();
        let mut primitives = Vec::new();
        for frame in 0..40 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(2.0 + frame as f64 / 60.0),
                    predicted_dt: 1.0 / 60.0,
                    ..Default::default()
                },
                |ctx| draw(ctx, 1.0),
            );
            gather_textures(&mut textures, &output.textures_delta);
            primitives = ctx.tessellate(output.shapes, 1.0);
        }
        let world = [0.20, 0.22, 0.26, 1.0];
        let image = rasterize(primitives, &textures, width, height, world);
        let flat = image::RgbaImage::from_fn(width as u32, height as u32, |x, y| {
            let pixel = image.get_pixel(x, y);
            let a = pixel[3] as f32 / 255.0;
            let over = |channel: usize| {
                ((pixel[channel] as f32 / 255.0 * a + world[channel] * (1.0 - a)) * 255.0).round()
                    as u8
            };
            image::Rgba([over(0), over(1), over(2), 255])
        });
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../screenshots");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("configs-window.png");
        flat.save(&path).expect("the configs preview saves");
        println!("configs window: {path:?}");
    }

     
     
    #[test]
    fn a_missing_configs_folder_lists_nothing() {
         
         
        for name in config::named_configs() {
            assert!(!name.is_empty(), "a listed config always has a name");
            assert!(!name.contains(['/', '\\']), "{name} cannot be a path");
        }
    }
}
