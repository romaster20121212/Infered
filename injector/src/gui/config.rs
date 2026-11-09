 
 
 
 
 
 
 
 
 
 
 
 

use std::collections::HashMap;

use egui::{
    pos2, vec2, Align, Align2, Color32, CursorIcon, FontId, Id, Layout, Margin, Rect, RichText,
    Rounding, ScrollArea, Sense, Stroke, TextEdit, TextureHandle, Ui, UiBuilder,
};

use super::theme::{lerp_color, pulse, with_alpha, Palette};
use super::{ease_out_cubic, paint_notice, widgets, InjectorGui};
use crate::modules::{ModuleView, SettingKind, SettingValue};

 
const PROFILE_W: f32 = 190.0;
 
const COLUMN_GAP: f32 = 16.0;
 
const ROW_H: f32 = 30.0;

 
 
#[derive(Default)]
struct RowChange {
    toggle: bool,
    expand: bool,
    settings: Vec<(String, SettingValue)>,
}

 
pub fn page(gui: &mut InjectorGui, ui: &mut Ui) {
    let now = ui.input(|input| input.time);
    let elapsed = (now - gui.config_opened_at).max(0.0);
    let t = (elapsed / 0.32).clamp(0.0, 1.0) as f32;
    let eased = t * t * (3.0 - 2.0 * t);
     
     
     
    let left = ease_out_cubic((elapsed / 0.34).clamp(0.0, 1.0) as f32);
    let right = ease_out_cubic(((elapsed - 0.05) / 0.34).clamp(0.0, 1.0) as f32);
    let rule = ease_out_cubic((elapsed / 0.46).clamp(0.0, 1.0) as f32);

     
     
    let (area, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
    let active = gui
        .config_modules
        .iter()
        .filter(|module| module.enabled)
        .count();
    let detail = format!("{active} on  ·  {} saved", gui.app.configs().len());
    paint_header(
        ui,
        area,
        &gui.config_icon,
        "CONFIG",
        "changes apply on the next injection",
        &detail,
        eased,
        rule,
        now,
    );

    let top = area.top() + 46.0 + 10.0 * (1.0 - eased);
    let height = (area.bottom() - top).max(80.0);
    let left_rect = Rect::from_min_size(pos2(area.left(), top), vec2(PROFILE_W, height))
        .translate(vec2(-12.0 * (1.0 - left), 0.0));
    let right_rect = Rect::from_min_size(
        pos2(area.left() + PROFILE_W + COLUMN_GAP, top),
        vec2((area.width() - PROFILE_W - COLUMN_GAP).max(140.0), height),
    )
    .translate(vec2(0.0, 12.0 * (1.0 - right)));

     
     
     
    {
        let mut ui = ui.new_child(
            UiBuilder::new()
                .max_rect(right_rect)
                .layout(Layout::top_down(Align::Min)),
        );
        ui.set_opacity(right);
        modules(gui, &mut ui, now);
    }
    {
        let mut ui = ui.new_child(
            UiBuilder::new()
                .max_rect(left_rect)
                .layout(Layout::top_down(Align::Min)),
        );
        ui.set_opacity(left);
        profiles(gui, &mut ui);
    }
}

 
 
 
pub(super) fn paint_header(
    ui: &Ui,
    area: Rect,
    icon: &TextureHandle,
    title: &str,
    hint: &str,
    detail: &str,
    opacity: f32,
    rule: f32,
    now: f64,
) {
    let mut painter = ui.painter().clone();
    painter.set_opacity(opacity);
    let icon_rect =
        Rect::from_min_size(pos2(area.left() + 1.0, area.top() + 1.0), vec2(22.0, 22.0));
    painter.image(
        icon.id(),
        icon_rect,
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
        Color32::WHITE,
    );
    painter.text(
        pos2(area.left() + 30.0, area.top()),
        Align2::LEFT_TOP,
        title,
        FontId::proportional(15.0),
        Palette::TEXT,
    );
    painter.text(
        pos2(area.left() + 30.0, area.top() + 19.0),
        Align2::LEFT_TOP,
        hint,
        FontId::proportional(9.0),
        with_alpha(Palette::TEXT_DIM, 0.62),
    );

     
     
    let summary = painter.layout_no_wrap(
        detail.to_uppercase(),
        FontId::proportional(9.0),
        with_alpha(Palette::ACCENT, 0.78),
    );
    let summary_x = area.right() - 2.0 - summary.size().x;
    painter.galley(pos2(summary_x, area.top() + 3.0), summary, Palette::ACCENT);
    painter.circle_filled(
        pos2(summary_x - 8.0, area.top() + 8.5),
        1.8,
        with_alpha(Palette::ACCENT, 0.8),
    );

     
     
    let y = area.top() + 38.0;
    let full_w = (area.width() - 2.0).max(0.0);
    painter.rect_filled(
        Rect::from_min_size(pos2(area.left() + 2.0, y), vec2(full_w, 1.0)),
        Rounding::same(1.0),
        with_alpha(Palette::TEXT_DIM, 0.12),
    );
    painter.rect_filled(
        Rect::from_min_size(pos2(area.left() + 2.0, y - 0.25), vec2(full_w * rule, 1.5)),
        Rounding::same(1.0),
        with_alpha(Palette::ACCENT, 0.35 + 0.25 * pulse(now, 3.4)),
    );
    painter.circle_filled(
        pos2(area.left() + 2.0 + full_w * rule, y),
        2.0,
        with_alpha(Palette::ACCENT_BRIGHT, 0.9 * (1.0 - rule)),
    );
}

 

fn profiles(gui: &mut InjectorGui, ui: &mut Ui) {
    ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
    section_label(ui, "PROFILES");

    let name = ui.add_sized(
        [ui.available_width(), 26.0],
        TextEdit::singleline(&mut gui.config_name)
            .hint_text("Config name")
            .margin(Margin::symmetric(8.0, 4.0)),
    );
    let entered = name.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
    if entered || pill_button(ui, "Save Config", ui.available_width(), 28.0).clicked() {
        gui.save_config_from_name();
    }
    if secondary_button(ui, "Load Config", ui.available_width(), 28.0).clicked() {
        gui.load_config_from_picker();
    }
    if let Some((message, ok)) = &gui.config_notice {
        paint_notice(ui, message, *ok);
    }

    ui.add_space(2.0);
    section_label(ui, "SAVED CONFIGS");
    let configs = gui.app.configs().to_vec();
    let mut load = None;
    let list_height = (ui.available_height() - 26.0).max(48.0);
    if configs.is_empty() {
        ui.label(
            RichText::new("Nothing saved yet")
                .size(10.5)
                .color(with_alpha(Palette::TEXT_DIM, 0.65)),
        );
    } else {
        ScrollArea::vertical()
            .id_salt("config_profiles")
            .auto_shrink([false, false])
            .max_height(list_height)
            .show(ui, |ui| {
                for (index, name) in configs.iter().enumerate() {
                    if profile_row(ui, name, index, gui.config_opened_at) {
                        load = Some(name.clone());
                    }
                }
            });
    }
    ui.label(
        RichText::new("Saved names replace the live setup when loaded.")
            .size(9.0)
            .color(with_alpha(Palette::TEXT_DIM, 0.5)),
    );

    if let Some(name) = load {
        gui.load_config_by_name(&name);
    }
}

 
 
fn profile_row(ui: &mut Ui, name: &str, index: usize, opened_at: f64) -> bool {
    let (slot, _) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::hover());
    let id = ui.id().with(("profile", name));
    let response = ui.interact(slot, id, Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(id.with("hover"), response.hovered(), 0.14);
    let now = ui.input(|input| input.time);
    let t = ((now - opened_at) / 0.35 - index as f64 * 0.05).clamp(0.0, 1.0) as f32;
    let appear = t * t * (3.0 - 2.0 * t);

    let mut painter = ui.painter().clone();
    painter.set_opacity(appear);
    let rect = slot.shrink2(vec2(0.0, 1.0));
    painter.rect_filled(
        rect,
        Rounding::same(7.0),
        lerp_color(Palette::CARD, Palette::CARD_HOVER, hover),
    );
    painter.rect_stroke(
        rect,
        Rounding::same(7.0),
        Stroke::new(1.0_f32, with_alpha(Palette::ACCENT, 0.08 + 0.30 * hover)),
    );
    let label = widgets::elided(
        ui,
        name,
        11.5,
        with_alpha(Palette::TEXT, 0.72 + 0.28 * hover),
        (slot.width() - 56.0).max(30.0),
    );
    painter.galley(
        pos2(slot.left() + 9.0, slot.center().y - label.size().y * 0.5),
        label,
        Palette::TEXT,
    );
    painter.text(
        pos2(slot.right() - 9.0, slot.center().y),
        Align2::RIGHT_CENTER,
        "LOAD",
        FontId::proportional(9.0),
        with_alpha(Palette::ACCENT, 0.35 + 0.6 * hover),
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response.clicked()
}

 

fn modules(gui: &mut InjectorGui, ui: &mut Ui, now: f64) {
    ui.spacing_mut().item_spacing = vec2(6.0, 5.0);
    let total = gui.config_modules.len();
    let active = gui
        .config_modules
        .iter()
        .filter(|module| module.enabled)
        .count();

    let (head, _) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::hover());
    let painter = ui.painter();
    painter.text(
        pos2(head.left() + 2.0, head.center().y),
        Align2::LEFT_CENTER,
        "MODULES",
        FontId::proportional(11.0),
        with_alpha(Palette::TEXT, 0.9),
    );
    painter.text(
        pos2(head.right() - 2.0, head.center().y),
        Align2::RIGHT_CENTER,
        format!("{active} on  ·  {total} total"),
        FontId::proportional(9.5),
        with_alpha(Palette::TEXT_DIM, 0.7),
    );

    ui.add_sized(
        [ui.available_width(), 26.0],
        TextEdit::singleline(&mut gui.config_search)
            .hint_text("Search modules or settings")
            .margin(Margin::symmetric(8.0, 4.0)),
    );

    if !gui.app.has_live_config() {
        warning_strip(
            ui,
            "No live config yet — start Minecraft once with Infered.",
        );
    } else if !gui.app.has_module_catalog() {
        warning_strip(ui, "Module names load after Infered's next run.");
    }
    if total == 0 {
        empty_modules(ui);
        return;
    }

    let query = gui.config_search.trim().to_lowercase();
    let visible: Vec<usize> = gui
        .config_modules
        .iter()
        .enumerate()
        .filter(|(_, module)| matches(module, &query))
        .map(|(index, _)| index)
        .collect();
    if visible.is_empty() {
        ui.add_space(14.0);
        ui.label(
            RichText::new("No module matches that search")
                .size(10.5)
                .color(with_alpha(Palette::TEXT_DIM, 0.7)),
        );
        return;
    }
    let groups = group_by_category(&gui.config_modules, &visible);

    let mut toggles: Vec<(String, bool)> = Vec::new();
    let mut expands: Vec<(String, bool)> = Vec::new();
    let mut settings: Vec<(String, String, SettingValue)> = Vec::new();

    ScrollArea::vertical()
        .id_salt("config_modules")
        .auto_shrink([false, false])
        .max_height(ui.available_height())
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 3.0);
            for (category, indices) in &groups {
                category_header(ui, category, indices.len());
                for &index in indices {
                    let id = gui.config_modules[index].id.clone();
                    let expanded = gui.config_expanded.contains(&id);
                    let change = module_row(
                        ui,
                        &mut gui.config_modules[index],
                        index,
                        expanded,
                        now,
                        gui.config_opened_at,
                        &mut gui.config_buffers,
                    );
                    if change.toggle {
                        toggles.push((id.clone(), gui.config_modules[index].enabled));
                    }
                    if change.expand {
                        expands.push((id.clone(), !expanded));
                    }
                    for (setting, value) in change.settings {
                        settings.push((id.clone(), setting, value));
                    }
                }
            }
            ui.add_space(4.0);
        });

    for (id, expanded) in expands {
        if expanded {
            gui.config_expanded.insert(id);
        } else {
            gui.config_expanded.remove(&id);
        }
    }
    for (id, enabled) in toggles {
        if let Err(error) = gui.app.set_module_enabled(&id, enabled) {
            gui.config_notice = Some((error, false));
        }
    }
    for (id, setting, value) in settings {
        if let Err(error) = gui.app.update_setting(&id, &setting, &value) {
            gui.config_notice = Some((error, false));
        }
    }
}

 
 
fn module_row(
    ui: &mut Ui,
    module: &mut ModuleView,
    index: usize,
    expanded: bool,
    now: f64,
    opened_at: f64,
    buffers: &mut HashMap<String, String>,
) -> RowChange {
    let (slot, _) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_H), Sense::hover());
    let id = ui.id().with(("module", module.id.as_str()));
    let row = ui.interact(slot, id, Sense::click());

    let hover =
        ui.ctx()
            .animate_bool_with_time(id.with("hover"), ui.rect_contains_pointer(slot), 0.14);
    let on = ui
        .ctx()
        .animate_bool_with_time(id.with("on"), module.enabled, 0.20);

    let switch =
        Rect::from_center_size(pos2(slot.right() - 20.0, slot.center().y), vec2(34.0, 17.0));
    let switch_response = ui.interact(switch, id.with("switch"), Sense::click());
    let switch_hover =
        ui.ctx()
            .animate_bool_with_time(id.with("switch_hover"), switch_response.hovered(), 0.12);

     
     
    let delay = (index as f64 * 0.004).min(0.30);
    let t = ((now - opened_at) / 0.35 - delay).clamp(0.0, 1.0) as f32;
    let appear = t * t * (3.0 - 2.0 * t);
    let mut painter = ui.painter().clone();
    painter.set_opacity(appear);

    let body = slot.shrink2(vec2(0.0, 1.0));
    painter.rect_filled(
        body,
        Rounding::same(8.0),
        with_alpha(Color32::WHITE, 0.028 * hover),
    );

     
    let dot = pos2(slot.left() + 7.0, slot.center().y);
    if module.enabled {
        painter.circle_filled(dot, 2.2, with_alpha(Palette::ACCENT, 0.85));
    } else {
        painter.circle_stroke(
            dot,
            2.0,
            Stroke::new(1.0_f32, with_alpha(Palette::TEXT_DIM, 0.5)),
        );
    }

    let name_x = slot.left() + 15.0;
    let text_w = (switch.left() - 28.0 - name_x).max(40.0);
    let name = widgets::elided(
        ui,
        &module.name,
        12.0,
        with_alpha(
            Palette::TEXT,
            if module.enabled {
                0.92
            } else {
                0.6 + 0.25 * hover
            },
        ),
        text_w,
    );
    painter.galley(
        pos2(name_x, slot.center().y - name.size().y * 0.5),
        name,
        Palette::TEXT,
    );

     
    if !module.settings.is_empty() {
        let open = ui
            .ctx()
            .animate_bool_with_time(id.with("chevron"), expanded, 0.15);
        let center = pos2(switch.left() - 13.0, slot.center().y);
        let ink = Stroke::new(1.2_f32, with_alpha(Palette::TEXT_DIM, 0.32 + 0.35 * hover));
        let end_y = -1.6 + 3.2 * open;
        let tip_y = 1.6 - 3.2 * open;
        painter.line_segment(
            [
                pos2(center.x - 3.2, center.y + end_y),
                pos2(center.x, center.y + tip_y),
            ],
            ink,
        );
        painter.line_segment(
            [
                pos2(center.x, center.y + tip_y),
                pos2(center.x + 3.2, center.y + end_y),
            ],
            ink,
        );
    }

    widgets::paint_toggle(&painter, switch, on, switch_hover);

    let mut change = RowChange::default();
    if switch_response.clicked() {
        module.enabled = !module.enabled;
        change.toggle = true;
    }
    if row.clicked() && !module.settings.is_empty() {
        change.expand = true;
    }
    if row.hovered() || switch_response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }

    if expanded && !module.settings.is_empty() {
        if !module.description.is_empty() {
            let (line, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), 18.0), Sense::hover());
            let text = widgets::elided(
                ui,
                &module.description,
                9.5,
                with_alpha(Palette::TEXT_DIM, 0.62),
                (line.width() - 30.0).max(60.0),
            );
            ui.painter().galley(
                pos2(line.left() + 18.0, line.top() + 2.0),
                text,
                Palette::TEXT_DIM,
            );
        }
        let module_id = module.id.clone();
        for setting in &mut module.settings {
            paint_setting(ui, &module_id, setting, buffers, &mut change);
        }
        ui.add_space(3.0);
    }

    change
}

 
 
 
fn paint_setting(
    ui: &mut Ui,
    module_id: &str,
    setting: &mut crate::modules::SettingView,
    buffers: &mut HashMap<String, String>,
    change: &mut RowChange,
) {
    ui.horizontal(|ui| {
        ui.add_space(18.0);
        ui.label(
            RichText::new(&setting.name)
                .size(11.0)
                .color(with_alpha(Palette::TEXT_DIM, 0.92)),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let id = ui.id().with(("setting", module_id, setting.name.as_str()));
            match (&setting.kind, &mut setting.value) {
                (SettingKind::Toggle, SettingValue::Toggle(on)) => {
                    if toggle_switch(ui, id, on) {
                        change
                            .settings
                            .push((setting.name.clone(), SettingValue::Toggle(*on)));
                    }
                }
                (SettingKind::Slider { min, max }, SettingValue::Slider(current)) => {
                    ui.spacing_mut().slider_width = 118.0;
                    let mut shown = *current;
                    let response =
                        ui.add(egui::Slider::new(&mut shown, *min..=*max).max_decimals(2));
                    if response.changed() {
                        *current = shown;
                    }
                    if response.drag_stopped() || (response.changed() && !response.dragged()) {
                        change
                            .settings
                            .push((setting.name.clone(), SettingValue::Slider(*current)));
                    }
                }
                (SettingKind::Choice { options }, SettingValue::Choice(selected)) => {
                    let before = *selected;
                    let label = options
                        .get(*selected)
                        .cloned()
                        .unwrap_or_else(|| format!("#{selected}"));
                    egui::ComboBox::from_id_salt(id)
                        .selected_text(RichText::new(label).size(10.5))
                        .width(132.0)
                        .show_ui(ui, |ui| {
                            for (index, option) in options.iter().enumerate() {
                                ui.selectable_value(selected, index, option);
                            }
                        });
                    if *selected != before {
                        change
                            .settings
                            .push((setting.name.clone(), SettingValue::Choice(*selected)));
                    }
                }
                (SettingKind::Color, SettingValue::Color(rgba)) => {
                    let mut color = color32(*rgba);
                    if ui.color_edit_button_srgba(&mut color).changed() {
                        *rgba = rgba_from(color);
                        change
                            .settings
                            .push((setting.name.clone(), SettingValue::Color(*rgba)));
                    }
                }
                (SettingKind::Text, SettingValue::Text(text)) => {
                    let key = format!("{module_id}/{}", setting.name);
                    let buffer = buffers.entry(key).or_insert_with(|| text.clone());
                    let response = ui.add_sized(
                        [168.0, 22.0],
                        TextEdit::singleline(buffer).margin(Margin::symmetric(6.0, 3.0)),
                    );
                    if response.lost_focus() && *buffer != *text {
                        *text = buffer.clone();
                        change
                            .settings
                            .push((setting.name.clone(), SettingValue::Text(text.clone())));
                    }
                }
                (SettingKind::BlockList, SettingValue::BlockList(list)) => {
                    let key = format!("{module_id}/{}", setting.name);
                    let buffer = buffers.entry(key).or_insert_with(|| list.join(", "));
                    let response = ui.add_sized(
                        [168.0, 22.0],
                        TextEdit::singleline(buffer)
                            .hint_text("minecraft:stone, …")
                            .margin(Margin::symmetric(6.0, 3.0)),
                    );
                    if response.lost_focus() {
                        let parsed: Vec<String> = buffer
                            .split([',', ';', '\n'])
                            .map(str::trim)
                            .filter(|entry| !entry.is_empty())
                            .map(str::to_string)
                            .collect();
                        if parsed != *list {
                            *list = parsed.clone();
                            change
                                .settings
                                .push((setting.name.clone(), SettingValue::BlockList(parsed)));
                        }
                    }
                }
                _ => {
                     
                     
                     
                    ui.label(
                        RichText::new("unavailable")
                            .size(9.5)
                            .color(with_alpha(Palette::ERR, 0.7)),
                    );
                }
            }
        });
    });
}

 
pub(super) fn toggle_switch(ui: &mut Ui, id: Id, value: &mut bool) -> bool {
    let (slot, _) = ui.allocate_exact_size(vec2(32.0, 16.0), Sense::hover());
    let response = ui.interact(slot, id, Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(id.with("hover"), response.hovered(), 0.12);
    let on = ui.ctx().animate_bool_with_time(id.with("on"), *value, 0.18);
    widgets::paint_toggle(ui.painter(), slot, on, hover);
    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    if response.clicked() {
        *value = !*value;
        true
    } else {
        false
    }
}

 

pub(super) fn section_label(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(9.5)
            .color(with_alpha(Palette::TEXT_DIM, 0.68)),
    );
}

 
pub(super) fn pill_button(ui: &mut Ui, text: &str, width: f32, height: f32) -> egui::Response {
    let (slot, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let response = ui.interact(slot, ui.id().with(("pill", text)), Sense::click());
    let hover =
        ui.ctx()
            .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.16);
    let press = ui.ctx().animate_bool_with_time(
        response.id.with("press"),
        response.is_pointer_button_down_on(),
        0.08,
    );
    let rect = Rect::from_center_size(slot.center() + vec2(0.0, press), slot.size());
    let painter = ui.painter();
    for (spread, alpha) in [(7.0, 0.05), (2.5, 0.10)] {
        painter.rect_filled(
            rect.expand(spread * (0.5 + 0.5 * hover)),
            Rounding::same(10.0),
            with_alpha(Palette::ACCENT, alpha * (0.4 + 0.6 * hover)),
        );
    }
    painter.rect_filled(
        rect,
        Rounding::same(8.0),
        lerp_color(Palette::ACCENT_DEEP, Palette::ACCENT, 0.55 + 0.45 * hover),
    );
    painter.rect_filled(
        Rect::from_min_size(
            rect.min + vec2(1.0, 1.0),
            vec2(rect.width() - 2.0, rect.height() * 0.45),
        ),
        Rounding {
            nw: 7.0,
            ne: 7.0,
            sw: 3.0,
            se: 3.0,
        },
        with_alpha(Color32::WHITE, 0.10 + 0.06 * hover),
    );
    painter.rect_stroke(
        rect,
        Rounding::same(8.0),
        Stroke::new(
            1.0_f32,
            with_alpha(Palette::ACCENT_BRIGHT, 0.30 + 0.45 * hover),
        ),
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::proportional(11.0),
        with_alpha(Color32::WHITE, 0.90 + 0.10 * hover),
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response
}

 
 
 
pub(super) fn secondary_button(ui: &mut Ui, text: &str, width: f32, height: f32) -> egui::Response {
    let (slot, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let response = ui.interact(slot, ui.id().with(("secondary", text)), Sense::click());
    let hover =
        ui.ctx()
            .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.16);
    let press = ui.ctx().animate_bool_with_time(
        response.id.with("press"),
        response.is_pointer_button_down_on(),
        0.08,
    );
    let rect = Rect::from_center_size(slot.center(), slot.size() * (1.0 - 0.02 * press));
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        Rounding::same(8.0),
        lerp_color(Palette::CARD, Palette::CARD_HOVER, hover),
    );
    painter.rect_stroke(
        rect,
        Rounding::same(8.0),
        Stroke::new(1.0_f32, with_alpha(Palette::ACCENT, 0.22 + 0.42 * hover)),
    );
    painter.rect_filled(
        Rect::from_min_size(
            pos2(rect.left() + 10.0, rect.bottom() - 1.5),
            vec2((rect.width() - 20.0) * hover, 1.5),
        ),
        Rounding::same(1.0),
        with_alpha(Palette::ACCENT, 0.85),
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::proportional(11.0),
        with_alpha(Palette::TEXT, 0.72 + 0.26 * hover),
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response
}

fn category_header(ui: &mut Ui, category: &str, count: usize) {
    let (slot, _) = ui.allocate_exact_size(vec2(ui.available_width(), 21.0), Sense::hover());
    let painter = ui.painter();
    painter.text(
        pos2(slot.left() + 7.0, slot.center().y),
        Align2::LEFT_CENTER,
        category.to_uppercase(),
        FontId::proportional(9.0),
        with_alpha(Palette::TEXT_DIM, 0.62),
    );
    painter.text(
        pos2(slot.right() - 4.0, slot.center().y),
        Align2::RIGHT_CENTER,
        count.to_string(),
        FontId::proportional(9.0),
        with_alpha(Palette::TEXT_DIM, 0.42),
    );
    painter.line_segment(
        [
            pos2(slot.left() + 7.0, slot.bottom() - 1.0),
            pos2(slot.right(), slot.bottom() - 1.0),
        ],
        Stroke::new(1.0_f32, with_alpha(Palette::TEXT_DIM, 0.08)),
    );
}

fn warning_strip(ui: &mut Ui, message: &str) {
    egui::Frame::none()
        .fill(with_alpha(Palette::ACCENT, 0.07))
        .stroke(Stroke::new(1.0_f32, with_alpha(Palette::ACCENT, 0.25)))
        .rounding(Rounding::same(6.0))
        .inner_margin(Margin::symmetric(8.0, 5.0))
        .show(ui, |ui| {
            ui.label(
                RichText::new(message)
                    .size(9.5)
                    .color(with_alpha(Palette::TEXT, 0.85)),
            );
        });
}

fn empty_modules(ui: &mut Ui) {
    let height = ui.available_height().max(120.0);
    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), height),
        Layout::top_down(Align::Center),
        |ui| {
            ui.add_space(28.0);
            ui.label(
                RichText::new("No module data yet")
                    .size(12.5)
                    .color(Palette::TEXT),
            );
            ui.label(
                RichText::new("Start Minecraft once with Infered, then reopen this page.")
                    .size(10.0)
                    .color(with_alpha(Palette::TEXT_DIM, 0.75)),
            );
        },
    );
}

 
 
fn matches(module: &ModuleView, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    module.name.to_lowercase().contains(query)
        || module.id.to_lowercase().contains(query)
        || module.category.to_lowercase().contains(query)
        || module.description.to_lowercase().contains(query)
        || module
            .settings
            .iter()
            .any(|setting| setting.name.to_lowercase().contains(query))
}

 
 
fn group_by_category(modules: &[ModuleView], visible: &[usize]) -> Vec<(String, Vec<usize>)> {
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    for &index in visible {
        let category = modules[index].category.clone();
        match groups.iter_mut().find(|(name, _)| *name == category) {
            Some((_, indices)) => indices.push(index),
            None => groups.push((category, vec![index])),
        }
    }
    groups.sort_by_key(|(name, _)| (category_rank(name), name.clone()));
    groups
}

fn category_rank(name: &str) -> usize {
    const ORDER: [&str; 9] = [
        "Combat",
        "Movement",
        "Render",
        "Player",
        "World",
        "Performance",
        "Misc",
        "Search",
        "Other",
    ];
    ORDER
        .iter()
        .position(|category| category.eq_ignore_ascii_case(name))
        .unwrap_or(ORDER.len())
}

 
 
fn color32(rgba: [f32; 4]) -> Color32 {
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color32::from_rgba_unmultiplied(
        channel(rgba[0]),
        channel(rgba[1]),
        channel(rgba[2]),
        channel(rgba[3]),
    )
}

fn rgba_from(color: Color32) -> [f32; 4] {
    [
        color.r() as f32 / 255.0,
        color.g() as f32 / 255.0,
        color.b() as f32 / 255.0,
        color.a() as f32 / 255.0,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::SettingView;
    use egui::{Context, Pos2, RawInput};

    fn module(name: &str, id: &str, enabled: bool) -> ModuleView {
        ModuleView {
            id: id.to_string(),
            name: name.to_string(),
            category: "Movement".to_string(),
            description: "Keeps you moving".to_string(),
            enabled,
            settings: Vec::new(),
        }
    }

    fn sprint() -> ModuleView {
        ModuleView {
            settings: vec![
                SettingView {
                    name: "Mode".to_string(),
                    kind: SettingKind::Choice {
                        options: vec!["Vanilla".to_string(), "Legit".to_string()],
                    },
                    value: SettingValue::Choice(1),
                },
                SettingView {
                    name: "Range".to_string(),
                    kind: SettingKind::Slider { min: 1.0, max: 6.0 },
                    value: SettingValue::Slider(4.5),
                },
            ],
            ..module("Sprint", "Sprint", true)
        }
    }

     
    pub(super) fn make_gui(modules: Vec<ModuleView>) -> InjectorGui {
        let mut gui = InjectorGui::stub_gui();
        gui.drawer_page = super::super::DrawerPage::Config;
        gui.config_modules = modules;
        gui
    }

     
    fn painted(gui: &mut InjectorGui) -> Vec<String> {
        let ctx = Context::default();
        super::super::theme::apply(&ctx);
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(702.0, 322.0));
         
        let _ = ctx.run(
            RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |_| {},
        );
        let output = ctx.run(
            RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(Palette::BG))
                    .show(ctx, |ui| page(gui, ui));
            },
        );
        output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect()
    }

    fn has(texts: &[String], expected: &str) -> bool {
        texts.iter().any(|text| text == expected)
    }

    #[test]
    fn the_page_shows_profiles_and_every_module() {
        let mut gui = make_gui(vec![sprint(), module("Fly", "Fly", false)]);
        let texts = painted(&mut gui);
        for expected in [
            "CONFIG",
            "PROFILES",
            "Save Config",
            "Load Config",
            "MODULES",
            "Sprint",
            "Fly",
            "MOVEMENT",
            "1 on  ·  2 total",
        ] {
            assert!(has(&texts, expected), "missing {expected:?} in {texts:?}");
        }
    }

    #[test]
    fn expanding_a_module_shows_its_settings() {
        let mut gui = make_gui(vec![sprint()]);
        gui.config_expanded.insert("Sprint".to_string());
        let texts = painted(&mut gui);
        for expected in ["Mode", "Range", "Legit"] {
            assert!(has(&texts, expected), "missing {expected:?} in {texts:?}");
        }
    }

    #[test]
    fn the_search_box_filters_the_module_list() {
        let mut gui = make_gui(vec![sprint(), module("Fly", "Fly", false)]);
        gui.config_search = "fly".to_string();
        let texts = painted(&mut gui);
        assert!(has(&texts, "Fly"), "the matching module stays");
        assert!(!has(&texts, "Sprint"), "the rest is filtered out");
    }

    #[test]
    fn without_a_catalog_the_page_explains_itself() {
        let mut gui = make_gui(Vec::new());
        let texts = painted(&mut gui);
        assert!(has(&texts, "No module data yet"), "got {texts:?}");
    }
}

#[cfg(test)]
mod preview {
    use super::*;
    use crate::modules::{SettingKind, SettingValue, SettingView};
    use egui::{Color32, Context, Pos2, RawInput, Rect, UiBuilder};
    use std::collections::HashMap;

    struct Tex {
        w: usize,
        h: usize,
        px: Vec<Color32>,
    }

    fn gather_textures(map: &mut HashMap<egui::TextureId, Tex>, delta: &egui::TexturesDelta) {
        for (id, image_delta) in &delta.set {
            let (w, h, pixels): (usize, usize, Vec<Color32>) = match &image_delta.image {
                egui::ImageData::Color(color) => {
                    (color.width(), color.height(), color.pixels.clone())
                }
                egui::ImageData::Font(font) => (
                    font.width(),
                    font.height(),
                    font.srgba_pixels(None).collect(),
                ),
            };
            if let Some([x0, y0]) = image_delta.pos {
                let Some(tex) = map.get_mut(id) else { continue };
                for y in 0..h {
                    for x in 0..w {
                        let (dx, dy) = (x0 + x, y0 + y);
                        if dx < tex.w && dy < tex.h {
                            tex.px[dy * tex.w + dx] = pixels[y * w + x];
                        }
                    }
                }
            } else {
                map.insert(*id, Tex { w, h, px: pixels });
            }
        }
    }

    fn sample(tex: Option<&Tex>, uv: Pos2) -> [f32; 4] {
        let Some(tex) = tex else { return [1.0; 4] };
        let x = (uv.x * tex.w as f32 - 0.5).clamp(0.0, tex.w as f32 - 1.0);
        let y = (uv.y * tex.h as f32 - 0.5).clamp(0.0, tex.h as f32 - 1.0);
        let (x0, y0) = (x.floor() as usize, y.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(tex.w - 1), (y0 + 1).min(tex.h - 1));
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let at = |x: usize, y: usize| {
            let c = tex.px[y * tex.w + x];
            [c.r() as f32, c.g() as f32, c.b() as f32, c.a() as f32]
        };
        let mut out = [0.0; 4];
        for (i, channel) in out.iter_mut().enumerate() {
            let top = at(x0, y0)[i] * (1.0 - fx) + at(x1, y0)[i] * fx;
            let bottom = at(x0, y1)[i] * (1.0 - fx) + at(x1, y1)[i] * fx;
            *channel = (top * (1.0 - fy) + bottom * fy) / 255.0;
        }
        out
    }

    fn rasterize(
        primitives: Vec<egui::ClippedPrimitive>,
        textures: &HashMap<egui::TextureId, Tex>,
        w: usize,
        h: usize,
    ) -> image::RgbaImage {
        let mut buffer = vec![[0.0f32; 4]; w * h];
        for clipped in primitives {
            let egui::epaint::Primitive::Mesh(mesh) = clipped.primitive else {
                continue;
            };
            let tex = textures.get(&mesh.texture_id);
            let clip = clipped.clip_rect;
            for triangle in mesh.indices.as_chunks::<3>().0 {
                let v = [
                    mesh.vertices[triangle[0] as usize],
                    mesh.vertices[triangle[1] as usize],
                    mesh.vertices[triangle[2] as usize],
                ];
                let area = (v[1].pos.x - v[0].pos.x) * (v[2].pos.y - v[0].pos.y)
                    - (v[2].pos.x - v[0].pos.x) * (v[1].pos.y - v[0].pos.y);
                if area.abs() < 1e-6 {
                    continue;
                }
                let min_x = v
                    .iter()
                    .map(|v| v.pos.x)
                    .fold(f32::MAX, f32::min)
                    .max(clip.left())
                    .floor()
                    .max(0.0) as usize;
                let max_x = v
                    .iter()
                    .map(|v| v.pos.x)
                    .fold(f32::MIN, f32::max)
                    .min(clip.right())
                    .ceil()
                    .min(w as f32 - 1.0) as usize;
                let min_y = v
                    .iter()
                    .map(|v| v.pos.y)
                    .fold(f32::MAX, f32::min)
                    .max(clip.top())
                    .floor()
                    .max(0.0) as usize;
                let max_y = v
                    .iter()
                    .map(|v| v.pos.y)
                    .fold(f32::MIN, f32::max)
                    .min(clip.bottom())
                    .ceil()
                    .min(h as f32 - 1.0) as usize;
                for y in min_y..=max_y {
                    for x in min_x..=max_x {
                        let p = pos2(x as f32 + 0.5, y as f32 + 0.5);
                        let w0 = ((v[1].pos.x - p.x) * (v[2].pos.y - p.y)
                            - (v[2].pos.x - p.x) * (v[1].pos.y - p.y))
                            / area;
                        let w1 = ((v[2].pos.x - p.x) * (v[0].pos.y - p.y)
                            - (v[0].pos.x - p.x) * (v[2].pos.y - p.y))
                            / area;
                        let w2 = 1.0 - w0 - w1;
                        if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                            continue;
                        }
                        let color = |i: usize| {
                            w0 * v[0].color[i] as f32
                                + w1 * v[1].color[i] as f32
                                + w2 * v[2].color[i] as f32
                        };
                        let uv = pos2(
                            w0 * v[0].uv.x + w1 * v[1].uv.x + w2 * v[2].uv.x,
                            w0 * v[0].uv.y + w1 * v[1].uv.y + w2 * v[2].uv.y,
                        );
                        let texel = sample(tex, uv);
                        let mut src = [0.0; 4];
                        for i in 0..4 {
                            src[i] = color(i) / 255.0 * texel[i];
                        }
                        let dst = &mut buffer[y * w + x];
                        for i in 0..4 {
                            dst[i] = src[i] + dst[i] * (1.0 - src[3]);
                        }
                    }
                }
            }
        }
        let mut image = image::RgbaImage::new(w as u32, h as u32);
        for (index, pixel) in buffer.iter().enumerate() {
            let a = pixel[3].clamp(0.0, 1.0);
            let channel = |i: usize| {
                if a <= 0.0 {
                    0
                } else {
                    ((pixel[i] / a).clamp(0.0, 1.0) * 255.0).round() as u8
                }
            };
            image.put_pixel(
                (index % w) as u32,
                (index / w) as u32,
                image::Rgba([channel(0), channel(1), channel(2), (a * 255.0) as u8]),
            );
        }
        image
    }

    fn setting(name: &str, kind: SettingKind, value: SettingValue) -> SettingView {
        SettingView {
            name: name.to_string(),
            kind,
            value,
        }
    }

    fn module(
        name: &str,
        category: &str,
        enabled: bool,
        description: &str,
        settings: Vec<SettingView>,
    ) -> crate::modules::ModuleView {
        crate::modules::ModuleView {
            id: name.to_string(),
            name: name.to_string(),
            category: category.to_string(),
            description: description.to_string(),
            enabled,
            settings,
        }
    }

    #[test]
    fn render_config_page_preview() {
        let (width, height) = (750usize, 450usize);
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(width as f32, height as f32));
        let drawer = Rect::from_min_size(pos2(24.0, 48.0), vec2(702.0, 354.0));

        let modules = vec![
            module(
                "KillAura",
                "Combat",
                true,
                "Hits the nearest target inside range",
                vec![
                    setting(
                        "Range",
                        SettingKind::Slider { min: 1.0, max: 6.0 },
                        SettingValue::Slider(3.0),
                    ),
                    setting(
                        "Targets",
                        SettingKind::Choice {
                            options: vec!["Players".into(), "Mobs".into(), "Both".into()],
                        },
                        SettingValue::Choice(0),
                    ),
                    setting(
                        "Through walls",
                        SettingKind::Toggle,
                        SettingValue::Toggle(true),
                    ),
                ],
            ),
            module(
                "AutoClicker",
                "Combat",
                true,
                "Clicks for you",
                vec![setting(
                    "CPS",
                    SettingKind::Slider {
                        min: 1.0,
                        max: 20.0,
                    },
                    SettingValue::Slider(12.0),
                )],
            ),
            module(
                "Sprint",
                "Movement",
                true,
                "Keeps you sprinting",
                vec![setting(
                    "Mode",
                    SettingKind::Choice {
                        options: vec!["Vanilla".into(), "Legit".into()],
                    },
                    SettingValue::Choice(1),
                )],
            ),
            module("Fly", "Movement", false, "Lets you fly", Vec::new()),
            module(
                "ESP",
                "Render",
                false,
                "Outlines players and storage",
                vec![setting(
                    "Color",
                    SettingKind::Color,
                    SettingValue::Color([0.88, 0.18, 0.2, 1.0]),
                )],
            ),
            module(
                "Xray",
                "Render",
                false,
                "Only draws selected ores",
                vec![setting(
                    "Blocks",
                    SettingKind::BlockList,
                    SettingValue::BlockList(vec![
                        "minecraft:diamond_ore".into(),
                        "minecraft:ancient_debris".into(),
                    ]),
                )],
            ),
            module(
                "AntiAFK",
                "Misc",
                true,
                "Moves so the server keeps you online",
                vec![setting(
                    "Message",
                    SettingKind::Text,
                    SettingValue::Text("still here".into()),
                )],
            ),
        ];

        let mut gui = super::tests::make_gui(modules);
        gui.app
            .stub_configs(vec!["crystal".into(), "legit".into(), "pvp".into()]);
        gui.config_expanded.insert("KillAura".to_string());

        let ctx = Context::default();
        super::super::theme::apply(&ctx);
        gui.config_icon =
            super::super::texture(&ctx, "preview_config", super::super::assets::CONFIG_PNG, 64);
        let warm = ctx.run(
            RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |_| {},
        );
        let mut textures = HashMap::new();
        gather_textures(&mut textures, &warm.textures_delta);

        let output = ctx.run(
            RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(Palette::BG))
                    .show(ctx, |ui| {
                        ui.painter()
                            .rect_filled(screen, Rounding::ZERO, Palette::BG);
                        ui.painter().rect_filled(
                            screen,
                            Rounding::ZERO,
                            Color32::from_black_alpha(90),
                        );
                        let mut ui = ui.new_child(
                            UiBuilder::new()
                                .max_rect(drawer)
                                .layout(Layout::top_down(Align::Min)),
                        );
                        egui::Frame::none()
                            .fill(Palette::BG)
                            .stroke(Stroke::new(1.0_f32, with_alpha(Palette::ACCENT, 0.20)))
                            .rounding(Rounding::same(16.0))
                            .inner_margin(Margin::same(16.0))
                            .show(&mut ui, |ui| {
                                ui.set_min_width(drawer.width() - 32.0);
                                ui.set_max_width(drawer.width() - 32.0);
                                ui.set_min_height(drawer.height() - 32.0);
                                ui.spacing_mut().item_spacing.y = 8.0;
                                let (row, _) = ui.allocate_exact_size(
                                    vec2(ui.available_width(), 30.0),
                                    Sense::hover(),
                                );
                                let painter = ui.painter();
                                let back = pos2(row.left() + 15.0, row.center().y);
                                painter.line_segment(
                                    [back + vec2(5.0, -6.0), back + vec2(-5.0, 0.0)],
                                    Stroke::new(1.4_f32, Palette::TEXT),
                                );
                                painter.line_segment(
                                    [back + vec2(-5.0, 0.0), back + vec2(5.0, 6.0)],
                                    Stroke::new(1.4_f32, Palette::TEXT),
                                );
                                let close = pos2(row.right() - 15.0, row.center().y);
                                painter.line_segment(
                                    [close + vec2(-4.5, -4.5), close + vec2(4.5, 4.5)],
                                    Stroke::new(1.4_f32, Palette::TEXT),
                                );
                                painter.line_segment(
                                    [close + vec2(4.5, -4.5), close + vec2(-4.5, 4.5)],
                                    Stroke::new(1.4_f32, Palette::TEXT),
                                );
                                ui.add_space(10.0);
                                page(&mut gui, ui);
                            });
                    });
            },
        );

         
         
         
        let content_left = drawer.left() + 16.0;
        let content_right = drawer.right() - 16.0;
        let column_break = content_left + PROFILE_W;
        let module_left = column_break + COLUMN_GAP;
        let mut texts = Vec::new();
        for clipped in &output.shapes {
            if let egui::Shape::Text(text) = &clipped.shape {
                let left = text.pos.x;
                let right = left + text.galley.size().x;
                let label = text.galley.text().to_owned();
                assert!(
                    left >= content_left - 0.5 && right <= content_right + 0.5,
                    "{label:?} spans {left:.1}..{right:.1}, outside {content_left:.1}..{content_right:.1}"
                );
                assert!(
                    right <= column_break + 0.5 || left >= module_left - 0.5,
                    "{label:?} straddles the column gap at {column_break:.1}..{module_left:.1}"
                );
                texts.push(label);
            }
        }
        assert!(texts.iter().any(|text| text == "MODULES"), "got {texts:?}");
        assert!(
            texts.iter().any(|text| text == "KillAura") && texts.iter().any(|text| text == "Range"),
            "the expanded module and its settings are missing: {texts:?}"
        );

        gather_textures(&mut textures, &output.textures_delta);
        let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
        let image = rasterize(primitives, &textures, width, height);
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../target/config-ui-preview.png");
        image.save(&path).expect("preview png saves");
        let lit = image
            .pixels()
            .filter(|p| p[0] as u32 + p[1] as u32 + p[2] as u32 > 40)
            .count();
        println!("config preview: {lit} lit pixels");
        assert!(lit > 20_000, "the preview is mostly empty");

         
         
        let ink = |x0: u32, x1: u32| {
            image
                .enumerate_pixels()
                .filter(|(x, _, pixel)| {
                    *x >= x0 && *x < x1 && pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32 > 150
                })
                .count()
        };
        let profiles = ink(content_left as u32, column_break as u32);
        let modules = ink(module_left as u32, content_right as u32);
        println!(
            "config preview: {profiles} profile-column pixels, {modules} module-column pixels"
        );
        assert!(profiles > 1_000, "the profiles column is blank");
        assert!(modules > 4_000, "the module column is blank");
    }

     
     
     
    #[test]
    fn render_menu_page_preview() {
        let (width, height) = (750usize, 450usize);
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(width as f32, height as f32));
        let drawer = Rect::from_min_size(pos2(24.0, 48.0), vec2(702.0, 354.0));

        let mut gui = super::tests::make_gui(Vec::new());
        gui.drawer_page = crate::gui::DrawerPage::Menu;
        gui.page_entered_at = -10.0;

        let ctx = Context::default();
        super::super::theme::apply(&ctx);
        let warm = ctx.run(
            RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |_| {},
        );
        let mut textures = HashMap::new();
        gather_textures(&mut textures, &warm.textures_delta);

        let output = ctx.run(
            RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(Palette::BG))
                    .show(ctx, |ui| {
                        ui.painter()
                            .rect_filled(screen, Rounding::ZERO, Palette::BG);
                        ui.painter().rect_filled(
                            screen,
                            Rounding::ZERO,
                            Color32::from_black_alpha(90),
                        );
                        let mut ui = ui.new_child(
                            UiBuilder::new()
                                .max_rect(drawer)
                                .layout(Layout::top_down(Align::Min)),
                        );
                        egui::Frame::none()
                            .fill(Palette::BG)
                            .stroke(Stroke::new(1.0_f32, with_alpha(Palette::ACCENT, 0.20)))
                            .rounding(Rounding::same(16.0))
                            .inner_margin(Margin::same(16.0))
                            .show(&mut ui, |ui| {
                                ui.set_min_width(drawer.width() - 32.0);
                                ui.set_max_width(drawer.width() - 32.0);
                                ui.set_min_height(drawer.height() - 32.0);
                                 
                                 
                                ui.add_space(30.0 + 14.0);
                                gui.paint_menu(ui, 10.0);
                            });
                    });
            },
        );

        let content_left = drawer.left() + 16.0;
        let content_right = drawer.right() - 16.0;
        let mut texts = Vec::new();
        for clipped in &output.shapes {
            if let egui::Shape::Text(text) = &clipped.shape {
                let left = text.pos.x;
                let right = left + text.galley.size().x;
                let label = text.galley.text().to_owned();
                assert!(
                    left >= content_left - 0.5 && right <= content_right + 0.5,
                    "{label:?} spans {left:.1}..{right:.1}, outside {content_left:.1}..{content_right:.1}"
                );
                texts.push(label);
            }
        }
         
         
        let joined = texts.join("");
        for expected in [
            "MENU",
            "COMMAND CENTER",
            "CONFIG",
            "FRIENDS",
            "MACROS",
            "ADDONS",
            "Modules, settings and profiles",
            "Players to leave alone",
            "Automate with .macro files",
            "Load .inf extensions",
        ] {
            assert!(
                joined.contains(expected),
                "missing {expected:?} in {joined:?}"
            );
        }
         
         
        for index in ["01", "02", "03", "04"] {
            assert!(joined.contains(index), "missing tile number {index}");
        }
        assert!(
            !joined.contains("OPEN"),
            "nothing is hovered in this preview"
        );

        gather_textures(&mut textures, &output.textures_delta);
        let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
        let image = rasterize(primitives, &textures, width, height);
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/menu-ui-preview.png");
        image.save(&path).expect("menu preview png saves");
        let lit = image
            .pixels()
            .filter(|p| p[0] as u32 + p[1] as u32 + p[2] as u32 > 40)
            .count();
        println!("menu preview: {lit} lit pixels");
        assert!(lit > 3_000, "the menu page is mostly empty");
    }

     
     
     
    #[test]
    fn render_friends_page_preview() {
        use crate::friends::{FriendMode, FriendView};

        let (width, height) = (750usize, 450usize);
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(width as f32, height as f32));
        let drawer = Rect::from_min_size(pos2(24.0, 48.0), vec2(702.0, 354.0));

        let mut gui = super::tests::make_gui(Vec::new());
        gui.drawer_page = crate::gui::DrawerPage::Friends;
        gui.friends_opened_at = -10.0;
        gui.friends_list = vec![
            FriendView {
                name: "Alex".into(),
                mode: FriendMode::Easy,
                protected: true,
                exempt: Vec::new(),
            },
            FriendView {
                name: "Steve".into(),
                mode: FriendMode::Advanced,
                protected: false,
                exempt: vec!["KillAura".into(), "TriggerBot".into()],
            },
            FriendView {
                name: "Notch".into(),
                mode: FriendMode::Easy,
                protected: false,
                exempt: Vec::new(),
            },
        ];
        gui.friends_expanded.insert("Alex".to_string());

        let ctx = Context::default();
        super::super::theme::apply(&ctx);
        gui.friends_icon = super::super::texture(
            &ctx,
            "preview_friends",
            super::super::assets::FRIENDS_PNG,
            64,
        );
        let warm = ctx.run(
            RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |_| {},
        );
        let mut textures = HashMap::new();
        gather_textures(&mut textures, &warm.textures_delta);

        let output = ctx.run(
            RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(Palette::BG))
                    .show(ctx, |ui| {
                        ui.painter()
                            .rect_filled(screen, Rounding::ZERO, Palette::BG);
                        ui.painter().rect_filled(
                            screen,
                            Rounding::ZERO,
                            Color32::from_black_alpha(90),
                        );
                        let mut ui = ui.new_child(
                            UiBuilder::new()
                                .max_rect(drawer)
                                .layout(Layout::top_down(Align::Min)),
                        );
                        egui::Frame::none()
                            .fill(Palette::BG)
                            .stroke(Stroke::new(1.0_f32, with_alpha(Palette::ACCENT, 0.20)))
                            .rounding(Rounding::same(16.0))
                            .inner_margin(Margin::same(16.0))
                            .show(&mut ui, |ui| {
                                ui.set_min_width(drawer.width() - 32.0);
                                ui.set_max_width(drawer.width() - 32.0);
                                ui.set_min_height(drawer.height() - 32.0);
                                ui.add_space(30.0 + 14.0);
                                crate::gui::friends::page(&mut gui, ui);
                            });
                    });
            },
        );

        let content_left = drawer.left() + 16.0;
        let content_right = drawer.right() - 16.0;
        let mut texts = Vec::new();
        for clipped in &output.shapes {
            if let egui::Shape::Text(text) = &clipped.shape {
                let left = text.pos.x;
                let right = left + text.galley.size().x;
                let label = text.galley.text().to_owned();
                assert!(
                    left >= content_left - 0.5 && right <= content_right + 0.5,
                    "{label:?} spans {left:.1}..{right:.1}, outside {content_left:.1}..{content_right:.1}"
                );
                texts.push(label);
            }
        }
         
        let joined = texts.join("");
        for expected in [
            "FRIENDS",
            "ADD FRIEND",
            "Alex",
            "Steve",
            "Notch",
            "Ignore all player modules",
            "All player modules ignored",
        ] {
            assert!(
                joined.contains(expected),
                "missing {expected:?} in {joined:?}"
            );
        }

        gather_textures(&mut textures, &output.textures_delta);
        let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
        let image = rasterize(primitives, &textures, width, height);
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../target/friends-ui-preview.png");
        image.save(&path).expect("friends preview png saves");
        let lit = image
            .pixels()
            .filter(|p| p[0] as u32 + p[1] as u32 + p[2] as u32 > 40)
            .count();
        println!("friends preview: {lit} lit pixels");
        assert!(lit > 3_000, "the friends page is mostly empty");
    }

     
     
     
    #[test]
    fn render_addons_page_preview() {
        use crate::app::AddonInfo;

        let (width, height) = (750usize, 450usize);
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(width as f32, height as f32));
        let drawer = Rect::from_min_size(pos2(24.0, 48.0), vec2(702.0, 354.0));

        let mut gui = super::tests::make_gui(Vec::new());
        gui.drawer_page = crate::gui::DrawerPage::Addons;
        gui.addons_opened_at = -10.0;
        gui.app.stub_addons(vec![
            AddonInfo {
                file_name: "combat_pack.inf".into(),
                title: "Combat Pack".into(),
                description: "Aimbot, reach and hit-select presets".into(),
                enabled: true,
            },
            AddonInfo {
                file_name: "movement_kit.inf".into(),
                title: "Movement Kit".into(),
                description: "Speed, jump and no-fall tuned for survival".into(),
                enabled: true,
            },
            AddonInfo {
                file_name: "auto_fish.inf".into(),
                title: "Auto Fish".into(),
                description: "Casts, watches the bobber and reels in".into(),
                enabled: false,
            },
        ]);

        let ctx = Context::default();
        super::super::theme::apply(&ctx);
        gui.addons_icon =
            super::super::texture(&ctx, "preview_addons", super::super::assets::ADDONS_PNG, 64);
        let warm = ctx.run(
            RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |_| {},
        );
        let mut textures = HashMap::new();
        gather_textures(&mut textures, &warm.textures_delta);

        let output = ctx.run(
            RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(Palette::BG))
                    .show(ctx, |ui| {
                        ui.painter()
                            .rect_filled(screen, Rounding::ZERO, Palette::BG);
                        ui.painter().rect_filled(
                            screen,
                            Rounding::ZERO,
                            Color32::from_black_alpha(90),
                        );
                        let mut ui = ui.new_child(
                            UiBuilder::new()
                                .max_rect(drawer)
                                .layout(Layout::top_down(Align::Min)),
                        );
                        egui::Frame::none()
                            .fill(Palette::BG)
                            .stroke(Stroke::new(1.0_f32, with_alpha(Palette::ACCENT, 0.20)))
                            .rounding(Rounding::same(16.0))
                            .inner_margin(Margin::same(16.0))
                            .show(&mut ui, |ui| {
                                ui.set_min_width(drawer.width() - 32.0);
                                ui.set_max_width(drawer.width() - 32.0);
                                ui.set_min_height(drawer.height() - 32.0);
                                ui.add_space(30.0 + 14.0);
                                crate::gui::addons::page(&mut gui, ui, 10.0);
                            });
                    });
            },
        );

        let content_left = drawer.left() + 16.0;
        let content_right = drawer.right() - 16.0;
        let mut texts = Vec::new();
        for clipped in &output.shapes {
            if let egui::Shape::Text(text) = &clipped.shape {
                let left = text.pos.x;
                let right = left + text.galley.size().x;
                let label = text.galley.text().to_owned();
                assert!(
                    left >= content_left - 0.5 && right <= content_right + 0.5,
                    "{label:?} spans {left:.1}..{right:.1}, outside {content_left:.1}..{content_right:.1}"
                );
                texts.push(label);
            }
        }
         
        let joined = texts.join("");
        for expected in [
            "ADDONS",
            "3 LOADED  ·  2 ACTIVE",
            "LOAD ADDON",
            "Combat Pack",
            "Aimbot, reach and hit-select presets",
            "Movement Kit",
            "Auto Fish",
        ] {
            assert!(
                joined.contains(expected),
                "missing {expected:?} in {joined:?}"
            );
        }

        gather_textures(&mut textures, &output.textures_delta);
        let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
        let image = rasterize(primitives, &textures, width, height);
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../target/addons-ui-preview.png");
        image.save(&path).expect("addons preview png saves");
        let lit = image
            .pixels()
            .filter(|p| p[0] as u32 + p[1] as u32 + p[2] as u32 > 40)
            .count();
        println!("addons preview: {lit} lit pixels");
        assert!(lit > 5_000, "the addons page is mostly empty");
    }

     
     
     
    #[test]
    fn render_macros_page_preview() {
        let (width, height) = (750usize, 450usize);
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(width as f32, height as f32));
        let drawer = Rect::from_min_size(pos2(24.0, 48.0), vec2(702.0, 354.0));

        let mut gui = super::tests::make_gui(Vec::new());
        gui.drawer_page = crate::gui::DrawerPage::Macros;
        gui.macros_page.stub_library(&[
            ("Strafe Combo", "F8", true, 4),
            ("Combo Opener", "interval", false, 6),
            ("Fishing Loop", "manual", true, 3),
        ]);
        gui.macros_page.stub_editor();

        let ctx = Context::default();
        super::super::theme::apply(&ctx);
        gui.macros_icon =
            super::super::texture(&ctx, "preview_macros", super::super::assets::MACROS_PNG, 64);
        let warm = ctx.run(
            RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |_| {},
        );
        let mut textures = HashMap::new();
        gather_textures(&mut textures, &warm.textures_delta);

        let output = ctx.run(
            RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(Palette::BG))
                    .show(ctx, |ui| {
                        ui.painter()
                            .rect_filled(screen, Rounding::ZERO, Palette::BG);
                        ui.painter().rect_filled(
                            screen,
                            Rounding::ZERO,
                            Color32::from_black_alpha(90),
                        );
                        let mut ui = ui.new_child(
                            UiBuilder::new()
                                .max_rect(drawer)
                                .layout(Layout::top_down(Align::Min)),
                        );
                        egui::Frame::none()
                            .fill(Palette::BG)
                            .stroke(Stroke::new(1.0_f32, with_alpha(Palette::ACCENT, 0.20)))
                            .rounding(Rounding::same(16.0))
                            .inner_margin(Margin::same(16.0))
                            .show(&mut ui, |ui| {
                                ui.set_min_width(drawer.width() - 32.0);
                                ui.set_max_width(drawer.width() - 32.0);
                                ui.set_min_height(drawer.height() - 32.0);
                                ui.add_space(30.0 + 14.0);
                                crate::gui::macros::page(&mut gui, ui);
                            });
                    });
            },
        );

        let content_left = drawer.left() + 16.0;
        let content_right = drawer.right() - 16.0;
        let mut texts = Vec::new();
        for clipped in &output.shapes {
            if let egui::Shape::Text(text) = &clipped.shape {
                let left = text.pos.x;
                let right = left + text.galley.size().x;
                let label = text.galley.text().to_owned();
                assert!(
                    left >= content_left - 0.5 && right <= content_right + 0.5,
                    "{label:?} spans {left:.1}..{right:.1}, outside {content_left:.1}..{content_right:.1}"
                );
                texts.push(label);
            }
        }
         
        let joined = texts.join("");
        for expected in [
            "MACROS",
            "3 MACROS  ·  2 ACTIVE",
            "NEW MACRO",
            "Strafe Combo",
            "Combo Opener",
            "Fishing Loop",
            "VISUAL",
            "SAVED",
            "TRIGGERS",
        ] {
            assert!(
                joined.contains(expected),
                "missing {expected:?} in {joined:?}"
            );
        }

        gather_textures(&mut textures, &output.textures_delta);
        let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
        let image = rasterize(primitives, &textures, width, height);
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../target/macros-ui-preview.png");
        image.save(&path).expect("macros preview png saves");
        let lit = image
            .pixels()
            .filter(|p| p[0] as u32 + p[1] as u32 + p[2] as u32 > 40)
            .count();
        println!("macros preview: {lit} lit pixels");
        assert!(lit > 6_000, "the macros page is mostly empty");
    }
}
