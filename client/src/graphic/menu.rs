 
 
 
 
 
 
 
 
 
 

use crate::graphic::ambience;
use crate::graphic::anim::{self, Easing, SpringCfg};
use crate::graphic::chams_video;
use crate::graphic::chams_warped;
use crate::graphic::configs;
use crate::graphic::controls;
use crate::graphic::hud::{comet_head, draw_top_bar_chip};
use crate::graphic::input::LAST_KEY_PRESSED;
use crate::graphic::notification::{Notification, NotificationType};
use crate::graphic::settings;
use crate::graphic::theme;
use crate::graphic::transparency::{self, Layer};
use crate::module::render::block_esp::{
    all_blocks, display_name, is_known_block, normalize_id, BLOCK_GROUPS, STORAGE_BLOCK_IDS,
};
use crate::module::render::chams::CLIP_SETTING as CHAMS_CLIP_SETTING;
use crate::module::render::xray::{is_ore_block, ORE_BLOCK_IDS};
use crate::module::{KeyboardKey, ModuleCategory, ModuleData, ModuleId, ModuleSetting, ModuleType};
use egui::{
    pos2, Align, Align2, Button, Color32, Context, FontId, Id, LayerId, Layout, Margin, Order,
    Painter, Pos2, Rect, RichText, Rounding, Sense, Shape, Stroke, Ui, Vec2,
};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

 
const PANEL_W: f32 = 168.0;
 
const GAP: f32 = 14.0;
 
const TITLE_H: f32 = 30.0;
 
const ROW_H: f32 = 24.0;
 
const SEARCH_BOX_H: f32 = 34.0;
 
const ROW_STRIDE: f32 = 320.0;
 
const ORIGIN_X: f32 = 24.0;
const ORIGIN_Y: f32 = 58.0;

 
pub(crate) type ModuleArc = Arc<Mutex<ModuleType>>;
 
pub(crate) type ModuleMap = HashMap<ModuleId, ModuleArc>;

 
 
 
 
 
fn modal_retreat(settings_progress: f32, friends_progress: f32, configs_progress: f32) -> f32 {
    let modal = if settings_progress.is_finite()
        && friends_progress.is_finite()
        && configs_progress.is_finite()
    {
        settings_progress
            .max(friends_progress)
            .max(configs_progress)
    } else {
        1.0
    };
    Easing::Out.apply(modal.clamp(0.0, 1.0))
}

 
 
 
 
 
 
#[derive(Clone, Copy, Debug)]
pub(crate) struct Chrome {
    pub(crate) progress: f32,
    pub(crate) retreat: f32,
}

impl Chrome {
     
    pub(crate) fn resting(progress: f32) -> Self {
        Self {
            progress,
            retreat: 0.0,
        }
    }

     
    fn interactive(self) -> bool {
        self.retreat <= 0.001
    }
}

 
 
 
 
 
pub fn draw(
    ctx: &Context,
    progress: f32,
    settings_progress: f32,
    friends_progress: f32,
    configs_progress: f32,
) {
    draw_backdrop(ctx, progress);
    draw_toolbar(ctx, progress);

     
     
     
     
    let retreat = modal_retreat(settings_progress, friends_progress, configs_progress);
    let chrome = Chrome {
        progress: progress.clamp(0.0, 1.0) * (1.0 - retreat),
        retreat,
    };
    if chrome.progress <= 0.001 {
        return;
    }

    let registry = crate::state::client().modules.by_id();

     
    let mut entries: Vec<(ModuleId, ModuleCategory)> = registry
        .iter()
        .map(|(id, arc)| {
            let module = arc.lock().unwrap();
            (*id, module.get_module_data().category)
        })
        .collect();
    entries.sort_by_key(|(id, _)| id.display_name());

     
     
    if crate::graphic::menu_sidebar::is_sidebar() || crate::graphic::menu_sidebar::is_clarity() {
        crate::graphic::menu_sidebar::draw(ctx, chrome, &entries, &registry);
        draw_addons_panel(ctx, chrome);
        return;
    }

     
    let all_members: Vec<Member> = entries
        .iter()
        .filter_map(|(id, _)| {
            registry.get(id).map(|arc| Member {
                name: id.display_name(),
                arc,
            })
        })
        .collect();

     
     
    let screen_w = ctx.screen_rect().width();
    let mut slot = Pos2::new(ORIGIN_X, ORIGIN_Y);
    let wrap = |slot: &mut Pos2| {
        if slot.x + PANEL_W > screen_w - 20.0 && slot.x > ORIGIN_X {
            slot.x = ORIGIN_X;
            slot.y += ROW_STRIDE;
        }
    };

    let categories = [
        ModuleCategory::Combat,
        ModuleCategory::Movement,
        ModuleCategory::Render,
        ModuleCategory::Visuals,
        ModuleCategory::Player,
        ModuleCategory::World,
        ModuleCategory::Performance,
        ModuleCategory::Misc,
        ModuleCategory::Macros,
         
         
        ModuleCategory::Dev,
    ];
    let mut panel_slot = 0;
    for category in categories {
        let members: Vec<Member> = entries
            .iter()
            .filter(|(_, cat)| *cat == category)
            .filter_map(|(id, _)| {
                registry.get(id).map(|arc| Member {
                    name: id.display_name(),
                    arc,
                })
            })
            .collect();
        if members.is_empty() {
            continue;
        }

        wrap(&mut slot);
        draw_panel(ctx, chrome, category, &members, slot, &registry, panel_slot);
        panel_slot += 1;
        slot.x += PANEL_W + GAP;
    }

     
    wrap(&mut slot);
    draw_search_panel(ctx, chrome, slot, &all_members, &registry, panel_slot);
    draw_addons_panel(ctx, chrome);
}

 
 
fn draw_addons_panel(ctx: &Context, chrome: Chrome) {
    let items = crate::addons::menu_items();
    let modules = crate::addons::modules();
    if items.is_empty() && modules.is_empty() {
        return;
    }
    let screen = ctx.screen_rect();
    egui::Area::new(Id::new("clickgui_addons"))
        .fixed_pos(pos2(screen.right() - 18.0, screen.bottom() - 22.0))
        .pivot(egui::Align2::RIGHT_BOTTOM)
        .order(Order::Foreground)
        .show(ctx, |ui| {
            ui.set_opacity(chrome.progress);
            if !chrome.interactive() {
                ui.disable();
            }
            let frame = egui::Frame::none()
                .fill(transparency::surface(1.0, Layer::Panel))
                .stroke(Stroke::new(1.0_f32, theme::BORDER))
                .rounding(Rounding::same(theme::RADIUS))
                .inner_margin(Margin::same(8.0))
                .show(ui, |ui| {
                    ui.set_min_width(PANEL_W);
                    ui.set_max_width(PANEL_W);
                    ui.label(
                        RichText::new("Addons")
                            .size(13.5)
                            .strong()
                            .color(theme::TEXT),
                    );
                    ui.add_space(3.0);
                    for item in &items {
                        ui.label(RichText::new(&item.label).size(12.5).color(theme::accent()));
                        ui.label(
                            RichText::new(format!("{} · {}", item.addon, item.description))
                                .size(10.5)
                                .color(theme::TEXT_DIM),
                        );
                        ui.add_space(3.0);
                    }
                    for (index, module) in modules.iter().enumerate() {
                        ui.label(
                            RichText::new(&module.category)
                                .size(11.0)
                                .color(theme::TEXT_MUTED),
                        );
                        let text =
                            RichText::new(&module.label)
                                .size(12.5)
                                .color(if module.enabled {
                                    theme::accent()
                                } else {
                                    theme::TEXT_DIM
                                });
                        if ui.add(Button::new(text).fill(theme::ELEVATED)).clicked() {
                            crate::addons::set_module_enabled(index, !module.enabled);
                        }
                        ui.label(
                            RichText::new(format!("{} · {}", module.addon, module.description))
                                .size(10.5)
                                .color(theme::TEXT_DIM),
                        );
                        ui.add_space(3.0);
                    }
                });
            transparency::edge(
                ui.painter(),
                frame.response.rect,
                Rounding::same(theme::RADIUS),
                Layer::Panel,
                chrome.progress,
            );
        });
}

 
 
 
const BACKDROP_DIM: f32 = 0.45;

 
 
 
fn backdrop_wash(progress: f32, reveal: f32) -> Color32 {
    let progress = if progress.is_finite() {
        progress.clamp(0.0, 1.0)
    } else {
        1.0
    };
    let reveal = if reveal.is_finite() {
        reveal.clamp(0.0, 1.0)
    } else {
        1.0
    };
    Color32::from_black_alpha((BACKDROP_DIM * reveal * 255.0 * progress).round() as u8)
}

 
 
 
 
fn draw_backdrop(ctx: &Context, progress: f32) {
    let progress = if progress.is_finite() {
        progress.clamp(0.0, 1.0)
    } else {
        1.0
    };
    let painter = ctx.layer_painter(LayerId::new(Order::Middle, Id::new("clickgui_backdrop")));
    let wash = backdrop_wash(progress, transparency::wash());
    if wash.a() > 0 {
        painter.rect_filled(ctx.screen_rect(), Rounding::ZERO, wash);
    }
}

 
const TOOLBAR_H: f32 = 30.0;
 
const TOOLBAR_GAP: f32 = 6.0;

 
const TOOLBAR_REVEAL_STAGGER: f32 = 0.05;
 
const TOOLBAR_REVEAL_SPAN: f32 = 0.6;
 
const TOOLBAR_REVEAL_DROP: f32 = 10.0;

 
#[derive(Clone, Copy)]
struct TileReveal {
     
    alpha: f32,
     
    dy: f32,
     
    slot: usize,
}

 
 
 
fn toolbar_reveal(progress: f32, slot: usize) -> TileReveal {
    let k =
        ((progress - slot as f32 * TOOLBAR_REVEAL_STAGGER) / TOOLBAR_REVEAL_SPAN).clamp(0.0, 1.0);
    let pop = Easing::OutBack.apply(k).clamp(0.0, 1.0);
    TileReveal {
        alpha: pop,
        dy: (1.0 - pop) * -TOOLBAR_REVEAL_DROP,
        slot,
    }
}

 
 
 
fn toolbar_button(
    ui: &mut Ui,
    text: &str,
    color: Color32,
    active: bool,
    reveal: TileReveal,
) -> egui::Response {
    let font = FontId::proportional(12.5);
    let text_w = ui.fonts(|f| {
        f.layout_no_wrap(text.to_string(), font.clone(), color)
            .size()
            .x
    });
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(text_w + 24.0, TOOLBAR_H), Sense::click());
    let ctx = ui.ctx().clone();
    let hover = anim::toggle(
        &ctx,
        ui.id().with("toolbar_hov").with(text),
        response.hovered(),
        0.12,
        Easing::Out,
    );
    let active_t = anim::toggle(
        &ctx,
        ui.id().with("toolbar_act").with(text),
        active,
        0.2,
        Easing::InOut,
    );
     
    let press = anim::toggle(
        &ctx,
        ui.id().with("toolbar_press").with(text),
        response.is_pointer_button_down_on(),
        0.08,
        Easing::Out,
    );
    let now = ctx.input(|i| i.time);
     
     
    let lift = hover * 1.2 - press * 1.1;
    let drawn = rect
        .shrink(press * 1.5)
        .translate(egui::vec2(0.0, reveal.dy - lift));
    let painter = ui.painter();
    let tile_radius = theme::RADIUS_INNER;
    draw_top_bar_chip(
        painter,
        drawn,
        tile_radius,
        reveal.alpha,
        comet_head(now, reveal.slot),
    );
     
    let round = Rounding::same(tile_radius);
    painter.rect_filled(
        drawn,
        round,
        Color32::from_white_alpha((hover * 14.0 * reveal.alpha).round() as u8),
    );
    painter.rect_filled(
        drawn,
        round,
        theme::with_alpha(theme::accent(), 0.2 * active_t * reveal.alpha),
    );
    painter.text(
        drawn.center(),
        Align2::CENTER_CENTER,
        text,
        font,
        theme::with_alpha(color, reveal.alpha),
    );
    click_ripple(ui, rect, &response);
    response
}

 
fn logo_tile(ui: &mut Ui, logo: &egui::TextureHandle, reveal: TileReveal) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(TOOLBAR_H), Sense::hover());
    let now = ui.ctx().input(|i| i.time);
    let rect = rect.translate(egui::vec2(0.0, reveal.dy));
    let painter = ui.painter();
     
     
    ambience::halo(
        painter,
        rect.center(),
        rect.width() * 0.95,
        theme::brand(),
        (0.10 + 0.04 * ((now as f32 * 1.1).sin())) * reveal.alpha,
    );
    draw_top_bar_chip(
        painter,
        rect,
        theme::RADIUS_INNER,
        reveal.alpha,
        comet_head(now, reveal.slot),
    );
     
    let size = logo.size_vec2();
    let inner = rect.shrink(5.0);
    let scale = (inner.width() / size.x).min(inner.height() / size.y);
    painter.image(
        logo.id(),
        Rect::from_center_size(rect.center(), size * scale),
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        theme::with_alpha(theme::brand(), reveal.alpha),
    );
}

 
fn draw_toolbar(ctx: &Context, progress: f32) {
     
    ctx.request_repaint();
    let slide = 16.0 * (1.0 - progress);
    egui::Area::new(Id::new("clickgui_toolbar"))
        .order(Order::Foreground)
        .anchor(Align2::CENTER_TOP, Vec2::new(0.0, 12.0 - slide))
        .show(ctx, |ui| {
            ui.set_opacity(progress);
             
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                match crate::graphic::branding::menu_logo_texture(ctx) {
                    Some(logo) => logo_tile(ui, &logo, toolbar_reveal(progress, 0)),
                    None => {
                        toolbar_button(
                            ui,
                            "Infered",
                            theme::accent(),
                            false,
                            toolbar_reveal(progress, 0),
                        );
                    }
                }
                ui.add_space(TOOLBAR_GAP);

                if toolbar_button(
                    ui,
                    "Panic",
                    theme::DANGER,
                    false,
                    toolbar_reveal(progress, 1),
                )
                .clicked()
                {
                    std::thread::spawn(crate::graphic::ui_engine::call_panic);
                }

                 
                 
                 
                 
                ui.add_space(TOOLBAR_GAP);
                let edit_on = crate::graphic::edit_ui::is_open();
                let edit_color = if edit_on {
                    theme::accent()
                } else {
                    theme::TEXT_DIM
                };
                if toolbar_button(
                    ui,
                    "Edit UI",
                    edit_color,
                    edit_on,
                    toolbar_reveal(progress, 2),
                )
                .clicked()
                {
                    crate::graphic::edit_ui::open();
                    crate::graphic::input::close_gui();
                }

                ui.add_space(TOOLBAR_GAP);
                if toolbar_button(
                    ui,
                    "Reset UI",
                    theme::TEXT_DIM,
                    false,
                    toolbar_reveal(progress, 3),
                )
                .clicked()
                {
                     
                     
                    ctx.memory_mut(|mem| mem.reset_areas());
                    ctx.data_mut(|data| data.clear());
                    crate::config::reset_ui_state();
                    crate::config::save();
                }

                ui.add_space(TOOLBAR_GAP);
                if toolbar_button(
                    ui,
                    "Reset Keybinds",
                    theme::TEXT_DIM,
                    false,
                    toolbar_reveal(progress, 4),
                )
                .clicked()
                {
                    crate::config::reset_keybinds();
                }

                ui.add_space(TOOLBAR_GAP);
                if toolbar_button(
                    ui,
                    "Reset Settings",
                    theme::TEXT_DIM,
                    false,
                    toolbar_reveal(progress, 5),
                )
                .clicked()
                {
                     
                     
                    crate::state::client().modules.reset_settings();
                    crate::config::save();
                }

                ui.add_space(TOOLBAR_GAP);
                let settings_open = settings::is_open();
                let settings_color = if settings_open {
                    theme::accent()
                } else {
                    theme::TEXT_DIM
                };
                if toolbar_button(
                    ui,
                    "Settings",
                    settings_color,
                    settings_open,
                    toolbar_reveal(progress, 6),
                )
                .clicked()
                {
                    settings::toggle_open();
                }

                 
                 
                ui.add_space(TOOLBAR_GAP);
                let configs_open = configs::is_open();
                let configs_color = if configs_open {
                    theme::accent()
                } else {
                    theme::TEXT_DIM
                };
                if toolbar_button(
                    ui,
                    "Configs",
                    configs_color,
                    configs_open,
                    toolbar_reveal(progress, 7),
                )
                .clicked()
                {
                    configs::toggle_open();
                }
            });
        });
}

 
const COLLAPSE_TIME: f32 = 0.28;
 
const COLLAPSE_BTN_RADIUS: f32 = 9.0;
 
const COLLAPSE_ICON_SIZE: f32 = 4.4;
 
const COLLAPSE_ICON_TIME: f32 = 0.34;

 
 
fn collapse_factor(ctx: &Context, category: ModuleCategory) -> f32 {
    anim::toggle(
        ctx,
        Id::new("panel_collapse").with(category.display_name()),
        crate::config::is_collapsed(category),
        COLLAPSE_TIME,
        Easing::InOut,
    )
}

 
const PANEL_REVEAL_STAGGER: f32 = 0.025;
 
const PANEL_REVEAL_SPAN: f32 = 0.7;
 
const PANEL_REVEAL_RISE: f32 = 18.0;

 
 
 
fn panel_reveal(progress: f32, slot: usize) -> f32 {
    let k = ((progress - slot as f32 * PANEL_REVEAL_STAGGER) / PANEL_REVEAL_SPAN).clamp(0.0, 1.0);
    Easing::Out.apply(k)
}

 
#[derive(Clone, Copy)]
struct Member<'a> {
    name: &'a str,
    arc: &'a ModuleArc,
}

 
fn draw_panel(
    ctx: &Context,
    chrome: Chrome,
    category: ModuleCategory,
    members: &[Member],
    slot: Pos2,
    registry: &ModuleMap,
    reveal_slot: usize,
) {
    let Chrome { progress, .. } = chrome;
    let name = category.display_name();

     
     
     
    let target = crate::config::panel_pos(category)
        .map(|p| Pos2::new(p[0], p[1]))
        .unwrap_or(slot);
    let pos = anim::spring_pos(
        ctx,
        Id::new("panel_pos").with(name),
        target,
        SpringCfg::PANEL,
    );

     
     
    let reveal = panel_reveal(progress, reveal_slot);
    let render_pos = pos + Vec2::new(0.0, PANEL_REVEAL_RISE * (1.0 - reveal));

    egui::Area::new(Id::new("clickgui_panel").with(name))
        .current_pos(render_pos)
        .order(Order::Foreground)
        .show(ctx, |ui| {
            ui.set_opacity(progress * reveal);
             
             
            if !chrome.interactive() {
                ui.disable();
            }
            let frame = egui::Frame::none()
                 
                 
                 
                .fill(transparency::surface(0.90, Layer::Panel))
                .stroke(Stroke::new(1.0_f32, theme::BORDER))
                .rounding(Rounding::same(theme::RADIUS))
                .shadow(ui.style().visuals.window_shadow)
                .show(ui, |ui| {
                     
                     
                     
                    ui.set_min_width(PANEL_W);
                    ui.set_max_width(PANEL_W);
                     
                     
                     
                     
                     
                    let max_rows_h =
                        (ctx.screen_rect().height() - render_pos.y - TITLE_H - 16.0).max(ROW_H);
                    ui.set_max_height(max_rows_h + TITLE_H + 8.0);

                    draw_title_bar(ui, category, Some(members.len()), true);

                     
                     
                    let open = 1.0 - collapse_factor(ctx, category);
                    if open > 0.001 {
                         
                         
                        let mut scroll = egui::style::ScrollStyle::floating();
                        scroll.bar_width = 5.0;
                        ui.style_mut().spacing.scroll = scroll;
                         
                         
                         
                        let active_fg = ui.visuals().widgets.active.fg_stroke.color;
                        ui.visuals_mut().widgets.active.fg_stroke.color = theme::accent();
                        egui::ScrollArea::vertical()
                            .id_salt(name)
                            .max_height(max_rows_h * open)
                            .auto_shrink([false, true])
                            .drag_to_scroll(false)
                            .show(ui, |ui| {
                                 
                                 
                                ui.visuals_mut().widgets.active.fg_stroke.color = active_fg;
                                ui.set_opacity(progress * reveal * open);
                                ui.set_min_width(PANEL_W);
                                ui.set_max_width(PANEL_W);
                                for member in members {
                                    draw_module_row(
                                        ui,
                                        member.name,
                                        member.arc,
                                        registry,
                                        PANEL_W,
                                        None,
                                        1.0,
                                    );
                                }
                                ui.add_space(6.0);
                            });
                    }
                });
            transparency::edge(
                ui.painter(),
                frame.response.rect,
                Rounding::same(theme::RADIUS),
                Layer::Panel,
                progress * reveal,
            );

             
             
            let held_id = Id::new("panel_held").with(name);
            let held = ctx.data(|d| d.get_temp::<bool>(held_id)).unwrap_or(false);
            let outline = anim::toggle(ctx, held_id.with("outline"), held, 0.12, Easing::Out);
            if outline > 0.001 {
                ui.painter().rect_stroke(
                    frame.response.rect.expand(2.0),
                    Rounding::same(theme::RADIUS + 2.0),
                    Stroke::new(
                        1.5_f32,
                        theme::with_alpha(theme::category_accent(category), 0.9 * outline),
                    ),
                );
            }
        });
}

 
 
 
 
 
 
fn draw_search_panel(
    ctx: &Context,
    chrome: Chrome,
    slot: Pos2,
    all: &[Member],
    registry: &ModuleMap,
    reveal_slot: usize,
) {
    let Chrome { progress, .. } = chrome;
    let name = ModuleCategory::Search.display_name();

     
     
    let target = crate::config::panel_pos(ModuleCategory::Search)
        .map(|p| Pos2::new(p[0], p[1]))
        .unwrap_or(slot);
    let pos = anim::spring_pos(
        ctx,
        Id::new("panel_pos").with(name),
        target,
        SpringCfg::PANEL,
    );
    let reveal = panel_reveal(progress, reveal_slot);
    let render_pos = pos + Vec2::new(0.0, PANEL_REVEAL_RISE * (1.0 - reveal));

     
     
    let query_id = Id::new("search_query");
    let mut query = ctx
        .data(|d| d.get_temp::<String>(query_id))
        .unwrap_or_default();

    egui::Area::new(Id::new("clickgui_panel").with(name))
        .current_pos(render_pos)
        .order(Order::Foreground)
        .show(ctx, |ui| {
            ui.set_opacity(progress * reveal);
             
            if !chrome.interactive() {
                ui.disable();
            }
            let frame = egui::Frame::none()
                .fill(transparency::surface(0.90, Layer::Panel))
                .stroke(Stroke::new(1.0_f32, theme::BORDER))
                .rounding(Rounding::same(theme::RADIUS))
                .shadow(ui.style().visuals.window_shadow)
                .show(ui, |ui| {
                    ui.set_min_width(PANEL_W);
                    ui.set_max_width(PANEL_W);
                     
                     
                    let max_rows_h =
                        (ctx.screen_rect().height() - render_pos.y - TITLE_H - SEARCH_BOX_H - 16.0)
                            .max(ROW_H);
                    ui.set_max_height(max_rows_h + TITLE_H + SEARCH_BOX_H + 8.0);

                    draw_title_bar(ui, ModuleCategory::Search, None, false);

                     
                    ui.add_space(6.0);
                    let changed = panel_search_field(ui, &mut query);
                    ui.add_space(6.0);

                     
                    let needle = query.trim().to_lowercase();
                    let results: Vec<Member> = all
                        .iter()
                        .filter(|member| search_matches(&needle, member.name) || (needle == "*" && crate::config::is_favorite(member.arc.lock().unwrap().get_module_data().id)))
                        .copied()
                        .collect();

                     
                    let mut scroll = egui::style::ScrollStyle::floating();
                    scroll.bar_width = 5.0;
                    ui.style_mut().spacing.scroll = scroll;
                    let active_fg = ui.visuals().widgets.active.fg_stroke.color;
                    ui.visuals_mut().widgets.active.fg_stroke.color = theme::accent();

                     
                     
                    let slide = if results.is_empty() {
                        0.0
                    } else {
                        search_slide(ctx, changed)
                    };

                    egui::ScrollArea::vertical()
                        .id_salt(name)
                        .max_height(max_rows_h)
                        .auto_shrink([false, true])
                        .drag_to_scroll(false)
                        .show(ui, |ui| {
                            ui.visuals_mut().widgets.active.fg_stroke.color = active_fg;
                            ui.set_min_width(PANEL_W);
                            ui.set_max_width(PANEL_W);
                             
                             
                             
                            ui.add_space(slide);
                             
                             
                            if !needle.is_empty() && results.is_empty() {
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new("No modules match.")
                                        .size(12.0)
                                        .color(theme::TEXT_MUTED),
                                );
                            }
                            for member in &results {
                                draw_module_row(
                                    ui,
                                    member.name,
                                    member.arc,
                                    registry,
                                    PANEL_W,
                                    None,
                                    1.0,
                                );
                            }
                            ui.add_space(6.0);
                        });
                });
            transparency::edge(
                ui.painter(),
                frame.response.rect,
                Rounding::same(theme::RADIUS),
                Layer::Panel,
                progress * reveal,
            );
        });

     
    ctx.data_mut(|d| d.insert_temp(query_id, query));
}

 
fn panel_search_field(ui: &mut Ui, query: &mut String) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        let response = ui.add(
            egui::TextEdit::singleline(query)
                .hint_text("Search modules…")
                .desired_width(PANEL_W - 78.0),
        );
        focus_glow(ui, &response);
        changed = response.changed();
        if ui.small_button("★").on_hover_text("Show favorites").clicked() {
            *query = "*".to_string();
            changed = true;
        }
        if !query.is_empty()
            && ui
                .add_sized([20.0, 20.0], Button::new("×"))
                .on_hover_text("Clear search")
                .clicked()
        {
            query.clear();
            response.request_focus();
            changed = true;
        }
    });
    changed
}

 
pub(crate) fn search_matches(needle: &str, module_name: &str) -> bool {
    let needle = needle.trim().to_lowercase();
    if needle.is_empty() {
        return false;
    }
    let name = module_name.to_lowercase();
    let compact: String = name.chars().filter(|c| !c.is_whitespace()).collect();
    name.contains(&needle) || needle.split_whitespace().all(|word| compact.contains(word))
}

 
const SEARCH_SLIDE_DIST: f32 = 10.0;
 
const SEARCH_SLIDE_TIME: f64 = 0.22;

 
 
 
 
 
fn search_slide(ctx: &Context, changed: bool) -> f32 {
    let now = ctx.input(|i| i.time);
    let id = Id::new("search_slide_start");
    let start = ctx.data(|d| d.get_temp::<f64>(id)).unwrap_or(now);
    let start = if changed { now } else { start };
    ctx.data_mut(|d| d.insert_temp(id, start));

    let t = ((now - start) / SEARCH_SLIDE_TIME).clamp(0.0, 1.0) as f32;
    SEARCH_SLIDE_DIST * (1.0 - Easing::Out.apply(t))
}

 
 
 
 
fn draw_title_bar(ui: &mut Ui, category: ModuleCategory, count: Option<usize>, collapsible: bool) {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(PANEL_W, TITLE_H), Sense::drag());
     
     
    let btn_rect = Rect::from_center_size(
        Pos2::new(rect.right() - 12.0, rect.center().y),
        Vec2::splat(18.0),
    );
    let collapse_btn = collapsible.then(|| {
        ui.interact(
            btn_rect,
            ui.id().with("title_collapse").with(category.display_name()),
            Sense::click(),
        )
    });
    let over_button = collapse_btn.as_ref().is_some_and(|btn| btn.hovered());
    if collapse_btn.as_ref().is_some_and(|btn| btn.clicked()) {
        let minimised = crate::config::is_collapsed(category);
        crate::config::set_collapsed(category, !minimised);
        crate::config::save();
    }
    if response.dragged() && !over_button {
        let delta = ui.ctx().input(|i| i.pointer.delta());
        let current = crate::config::panel_pos(category)
            .map(|p| Pos2::new(p[0], p[1]))
            .unwrap_or(rect.min);
        let next = current + delta;
        crate::config::set_panel_pos(category, [next.x, next.y]);
    }

    let ctx = ui.ctx().clone();
     
     
    let held = response.is_pointer_button_down_on() && !over_button;
    ctx.data_mut(|d| d.insert_temp(Id::new("panel_held").with(category.display_name()), held));
    if response.dragged() {
        ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
    } else if response.hovered() {
        ctx.set_cursor_icon(egui::CursorIcon::Grab);
    }
    let hover = anim::toggle(
        &ctx,
        ui.id().with("title_hov").with(category.display_name()),
        response.hovered() || response.dragged(),
        0.12,
        Easing::Out,
    );

     
    let collapse = if collapsible {
        collapse_factor(&ctx, category)
    } else {
        0.0
    };
     
    let centre_y = rect.center().y;
    let category_tint = theme::category_accent(category);
    {
        let painter = ui.painter();
        painter.rect_filled(
            rect,
            Rounding {
                nw: theme::RADIUS,
                ne: theme::RADIUS,
                sw: theme::RADIUS * collapse,
                se: theme::RADIUS * collapse,
            },
            theme::lerp_color(theme::ELEVATED, theme::SURFACE_HOVER, hover),
        );

         
         
         
        let pulse = 0.5 + 0.5 * ((ctx.input(|i| i.time) as f32 * 1.6).sin());
        let pip_h = 12.0 + 1.2 * pulse + 1.8 * hover;
        painter.rect_filled(
            Rect::from_min_size(
                Pos2::new(rect.left() + 9.0, centre_y - pip_h * 0.5),
                Vec2::new(3.0, pip_h),
            ),
            Rounding::same(1.5),
            theme::with_alpha(category_tint, 0.84 + 0.16 * pulse),
        );
         
         
        paint_category_glyph(
            painter,
            Pos2::new(rect.left() + 22.0, centre_y),
            category,
            theme::lerp_color(
                theme::with_alpha(category_tint, 0.72),
                theme::TEXT,
                hover * 0.55,
            ),
        );
        painter.text(
            Pos2::new(rect.left() + 33.0, centre_y),
            Align2::LEFT_CENTER,
            category.display_name(),
            FontId::proportional(13.5),
            theme::TEXT,
        );
    }

     
     
     
     
     
    let grip_x = rect.right() - 12.0;
    if let Some(btn) = &collapse_btn {
        click_ripple(ui, btn_rect, btn);
        paint_collapse_control(ui.painter(), &ctx, btn_rect, btn, collapse);
    }

    {
        let painter = ui.painter();
        if collapse_btn.is_none() {
            for dy in [-4.0, 0.0, 4.0] {
                painter.circle_filled(Pos2::new(grip_x, centre_y + dy), 1.1, theme::TEXT_MUTED);
            }
        }
        if let Some(count) = count {
            let galley = painter.layout_no_wrap(
                count.to_string(),
                FontId::proportional(11.0),
                theme::TEXT_DIM,
            );
            let size = galley.size();
            let pill = Rect::from_min_size(
                Pos2::new(grip_x - 10.0 - size.x - 9.0, centre_y - 7.0),
                Vec2::new(size.x + 9.0, 14.0),
            );
            painter.rect_filled(pill, Rounding::same(7.0), theme::SURFACE);
            painter.galley(pill.center() - size / 2.0, galley, theme::TEXT_DIM);
        }

         
         
         
        let line = Rect::from_min_size(
            rect.left_bottom() - Vec2::new(0.0, 2.0),
            Vec2::new(accent_line_len(collapse), 2.0),
        );
        let solid = category_tint;
        let clear = Color32::from_rgba_unmultiplied(solid.r(), solid.g(), solid.b(), 0);
        let mut mesh = egui::Mesh::default();
        mesh.colored_vertex(line.left_top(), solid);
        mesh.colored_vertex(line.right_top(), clear);
        mesh.colored_vertex(line.right_bottom(), clear);
        mesh.colored_vertex(line.left_bottom(), solid);
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        painter.add(Shape::mesh(mesh));
    }
}

 
 
 
 
 
pub(crate) fn paint_category_glyph(
    painter: &Painter,
    center: Pos2,
    category: ModuleCategory,
    color: Color32,
) {
    paint_category_glyph_scaled(painter, center, category, color, 1.0);
}

 
 
 
pub(crate) fn paint_category_glyph_scaled(
    painter: &Painter,
    center: Pos2,
    category: ModuleCategory,
    color: Color32,
    scale: f32,
) {
    let s = if scale.is_finite() {
        scale.max(0.05)
    } else {
        1.0
    };
    let stroke = Stroke::new(1.4_f32 * s, color);
    let at = |x: f32, y: f32| center + Vec2::new(x * s, y * s);
    let dot = |r: f32| r * s;
    match category {
        ModuleCategory::Combat => {
             
            painter.line_segment([at(-3.4, -3.4), at(3.4, 3.4)], stroke);
            painter.line_segment([at(3.4, -3.4), at(-3.4, 3.4)], stroke);
        }
        ModuleCategory::Movement => {
             
            for dx in [-2.2_f32, 1.4] {
                paint_rounded_polyline(
                    painter,
                    &[at(dx - 1.6, -3.6), at(dx + 1.8, 0.0), at(dx - 1.6, 3.6)],
                    stroke,
                );
            }
        }
        ModuleCategory::Render => {
             
            painter.circle_stroke(center, dot(3.7), stroke);
            painter.circle_filled(center, dot(1.3), color);
        }
        ModuleCategory::Visuals => {
             
            paint_rounded_polyline(
                painter,
                &[
                    at(0.0, -4.0),
                    at(3.0, 0.0),
                    at(0.0, 4.0),
                    at(-3.0, 0.0),
                    at(0.0, -4.0),
                ],
                stroke,
            );
            painter.circle_filled(at(3.8, -3.4), dot(0.9), color);
        }
        ModuleCategory::Player => {
             
            painter.circle_stroke(at(0.0, -2.5), dot(2.0), stroke);
            paint_rounded_polyline(
                painter,
                &[
                    at(-3.6, 3.9),
                    at(-3.6, 2.3),
                    at(0.0, 0.8),
                    at(3.6, 2.3),
                    at(3.6, 3.9),
                ],
                stroke,
            );
        }
        ModuleCategory::World => {
             
            painter.circle_stroke(center, dot(3.9), stroke);
            painter.line_segment([at(-3.7, 0.0), at(3.7, 0.0)], stroke);
            painter.line_segment([at(0.0, -3.7), at(0.0, 3.7)], stroke);
        }
        ModuleCategory::Performance => {
             
            painter.add(Shape::convex_polygon(
                vec![
                    at(1.0, -4.2),
                    at(-3.0, 0.6),
                    at(-0.3, 0.6),
                    at(-1.4, 4.2),
                    at(3.0, -0.9),
                    at(0.2, -0.9),
                ],
                color,
                Stroke::NONE,
            ));
        }
        ModuleCategory::Misc => {
             
            for dx in [-2.6_f32, 0.0, 2.6] {
                painter.circle_filled(at(dx, 0.0), dot(0.95), color);
            }
        }
        ModuleCategory::Macros => {
             
            paint_rounded_polyline(
                painter,
                &[
                    at(-3.6, -3.2),
                    at(3.6, -3.2),
                    at(3.6, 3.2),
                    at(-3.6, 3.2),
                    at(-3.6, -3.2),
                ],
                stroke,
            );
            painter.line_segment([at(-1.4, 0.6), at(1.4, 0.6)], stroke);
        }
        ModuleCategory::Search => {
             
            painter.circle_stroke(at(-0.8, -0.8), dot(3.0), stroke);
            painter.line_segment([at(1.5, 1.5), at(3.9, 3.9)], stroke);
        }
        ModuleCategory::Dev => {
             
            paint_rounded_polyline(
                painter,
                &[at(-3.6, -3.2), at(-0.4, 0.0), at(-3.6, 3.2)],
                stroke,
            );
            painter.line_segment([at(0.8, 3.0), at(3.6, 3.0)], stroke);
        }
    }
}

 
 
 
fn accent_line_len(collapse: f32) -> f32 {
    PANEL_W * (0.58 + 0.42 * collapse.clamp(0.0, 1.0))
}

 
 
 
 
 
fn collapse_chevron(centre: Pos2, turn: f32, size: f32) -> [Pos2; 3] {
    let angle = std::f32::consts::PI * turn;
    let (sin, cos) = angle.sin_cos();
    let rotate = |v: Vec2| Vec2::new(v.x * cos - v.y * sin, v.x * sin + v.y * cos);
    [
        Vec2::new(size, size * 0.58),
        Vec2::new(0.0, -size * 0.58),
        Vec2::new(-size, size * 0.58),
    ]
    .map(|v| centre + rotate(v))
}

 
 
 
fn paint_rounded_polyline(painter: &Painter, points: &[Pos2], stroke: Stroke) {
    for pair in points.windows(2) {
        painter.line_segment([pair[0], pair[1]], stroke);
    }
    for point in points {
        painter.circle_filled(*point, stroke.width * 0.5, stroke.color);
    }
}

 
 
 
 
 
 
 
fn paint_collapse_control(
    painter: &Painter,
    ctx: &Context,
    btn_rect: Rect,
    btn: &egui::Response,
    collapse: f32,
) {
    let hover = anim::toggle(ctx, btn.id.with("hov"), btn.hovered(), 0.14, Easing::Out);
     
    let press = anim::spring(
        ctx,
        btn.id.with("press"),
        if btn.is_pointer_button_down_on() {
            1.0
        } else {
            0.0
        },
        SpringCfg::SNAPPY,
    )
    .clamp(0.0, 1.5);
    let turn = anim::toggle(
        ctx,
        btn.id.with("turn"),
        collapse > 0.5,
        COLLAPSE_ICON_TIME,
        Easing::OutBack,
    );
    let tint = turn.clamp(0.0, 1.0);

    let scale = 1.0 + 0.10 * hover - 0.16 * press;
    let centre = btn_rect.center();
    let radius = COLLAPSE_BTN_RADIUS * scale;

     
     
    let halo = theme::lerp_color(theme::SURFACE_HOVER, theme::accent(), tint * 0.5);
    for (grow, strength) in [(2.4_f32, 0.12_f32), (1.2, 0.24), (0.0, 0.30)] {
        let alpha = (strength * (0.25 + 0.75 * hover) * (1.0 + 0.35 * tint)).min(1.0);
        painter.circle_filled(centre, radius + grow, theme::with_alpha(halo, alpha));
    }
    painter.circle_stroke(
        centre,
        radius,
        Stroke::new(
            1.0_f32,
            theme::lerp_color(theme::BORDER, theme::accent(), 0.25 + 0.75 * tint),
        ),
    );

    paint_rounded_polyline(
        painter,
        &collapse_chevron(centre, turn, COLLAPSE_ICON_SIZE * scale),
        Stroke::new(
            1.7_f32,
            theme::lerp_color(
                theme::lerp_color(theme::TEXT_MUTED, theme::TEXT, hover),
                theme::accent(),
                tint * 0.85,
            ),
        ),
    );
}

 
 
 
 
 
 
pub(crate) fn draw_module_row(
    ui: &mut Ui,
    name: &str,
    arc: &ModuleArc,
    registry: &ModuleMap,
    width: f32,
    hint: Option<ModuleCategory>,
    scale: f32,
) {
    let mut module = arc.lock().unwrap();
    let enabled = module.get_module_data().enabled;
    let id = module.get_module_data().id;
     
     
    let tint = theme::category_accent(module.get_module_data().category);

     
     
     
    let row_h = ROW_H * scale;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, row_h), Sense::click());
    let arrow_zone = Rect::from_min_size(
        Pos2::new(rect.max.x - 24.0 * scale, rect.min.y),
        Vec2::new(24.0 * scale, row_h),
    );
     
     
     
     
    let arrow_response = ui.interact(
        arrow_zone,
        ui.id().with("row_arrow").with(name),
        Sense::click(),
    );
     
     
     
    let star_zone = Rect::from_min_size(
        Pos2::new(rect.max.x - 47.0 * scale, rect.min.y),
        Vec2::new(23.0 * scale, row_h),
    );
    let star_response = ui.interact(star_zone, ui.id().with("row_favorite").with(name), Sense::click())
        .on_hover_text("Toggle favorite");
    let favorite = crate::config::is_favorite(id);
    let favorite_changed = star_response.clicked();
    if favorite_changed {
        crate::config::set_favorite(id, !favorite);
    }
    let expanded = crate::config::is_expanded(id);
    let mut collapse = egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        ui.id().with("row_collapse").with(name),
        expanded,
    );

    let ctx = ui.ctx();
    let hover = anim::toggle(
        ctx,
        ui.id().with("row_hov").with(name),
        response.hovered(),
        0.12,
        Easing::Out,
    );
    let enable = anim::toggle(
        ctx,
        ui.id().with("row_en").with(name),
        enabled,
        0.18,
        Easing::Out,
    );
     
     
    let magnet = ctx
        .pointer_latest_pos()
        .filter(|pointer| rect.expand(6.0).contains(*pointer))
        .map_or(0.0, |pointer| {
            ((pointer.x - rect.center().x) / (rect.width() * 0.5)).clamp(-1.0, 1.0)
        })
        * hover
        * 2.0
        * scale;
     
    let nudge = hover * 1.2 + enable * 0.6 + magnet;

     
    {
        let painter = ui.painter();
        let tile = rect;
        let round = Rounding::ZERO;
        painter.rect_filled(
            tile,
            round,
            theme::lerp_color(Color32::TRANSPARENT, theme::SURFACE_HOVER, hover),
        );
         
         
        painter.rect_filled(tile, round, theme::with_alpha(tint, 0.018 * enable));
         
         
         
        let gradient = crate::graphic::module_gradient::is_on()
            && enable > 0.001
            && crate::graphic::module_gradient::paint(
                painter,
                tile,
                tint,
                crate::graphic::module_gradient::seed(name),
                ctx.input(|i| i.time) as f32,
                enable,
            );
        if enable > 0.001 {
             
             
            let bar = Rect::from_center_size(
                Pos2::new(tile.min.x + (1.5 + 0.25 * enable) * scale, tile.center().y),
                Vec2::new(
                    (3.0 + 0.5 * enable) * scale,
                    (row_h - 6.0 * scale).max(1.0) * enable,
                ),
            );
            painter.rect_filled(bar, Rounding::same(2.0 * scale), tint);
        }
        let text_color = theme::lerp_color(theme::TEXT_DIM, theme::TEXT, hover.max(enable));
        let name_font = FontId::proportional(13.0 * scale);
        let name_pos = Pos2::new(rect.min.x + 12.0 * scale + nudge, rect.center().y);
        if gradient {
             
             
            painter.text(
                name_pos + Vec2::new(0.0, 1.0 * scale),
                Align2::LEFT_CENTER,
                name,
                name_font.clone(),
                crate::graphic::module_gradient::text_shadow(),
            );
        }
        painter.text(
            name_pos,
            Align2::LEFT_CENTER,
            name,
            name_font.clone(),
            if gradient { theme::TEXT } else { text_color },
        );
        if let Some(category) = hint {
            let name_width = ui.fonts(|fonts| {
                fonts
                    .layout_no_wrap(name.to_owned(), name_font, Color32::WHITE)
                    .size()
                    .x
            });
            paint_category_glyph_scaled(
                painter,
                Pos2::new(
                    rect.min.x + 22.0 * scale + nudge + name_width,
                    rect.center().y,
                ),
                category,
                theme::with_alpha(tint, 0.35 + 0.45 * hover),
                0.72 * scale,
            );
        }
    }

    ui.painter().text(star_zone.center(), Align2::CENTER_CENTER, if favorite { "★" } else { "☆" },
        FontId::proportional(15.0 * scale), if favorite { tint } else { theme::TEXT_MUTED });
    click_ripple(ui, rect, &response);
    click_ripple(ui, arrow_zone, &arrow_response);

     
     
     
    if arrow_response.clicked() || response.secondary_clicked() {
        crate::config::set_expanded(id, !expanded);
    } else if response.clicked() && !favorite_changed {
        let next = !enabled;
        module.get_module_data_mut().set_enabled(next);
         
         
         
        if id == ModuleId::MenuHud && !next {
            crate::graphic::input::request_gui_close();
        }
         
         
        if crate::state::client().modules.is_active() {
            let _ = if next {
                module.on_start()
            } else {
                module.on_stop()
            };
        }
         
        let now = ui.input(|i| i.time);
        ui.data_mut(|data| {
            data.insert_temp(Id::new("row_sweep").with(name), (now, next));
        });
    }

    drop(module);
    let mut module = arc.lock().unwrap();

    paint_row_sweep(ui, rect, name, tint);

     
     
    let description = module.get_module_data().description.clone();
    crate::graphic::tooltips::delayed_tooltip(
        ui.ctx(),
        ui.id().with("module_tip").with(name),
        response.layer_id,
        response.hovered() || arrow_response.hovered(),
        name,
        &description,
    );

     
     
     
    collapse.set_open(crate::config::is_expanded(id));
    let openness = collapse.openness(ui.ctx());
    let chevron_color = if arrow_response.hovered() {
        theme::TEXT
    } else {
        theme::TEXT_MUTED
    };
    paint_chevron(
        ui.painter(),
        arrow_zone.center() + Vec2::new(nudge * 0.6, 0.0),
        openness,
        chevron_color,
        scale,
    );

     
     
    let mut reset_requested = false;
    collapse.show_body_unindented(ui, |ui| {
        reset_requested = draw_settings(
            ui,
            module.get_module_data_mut(),
            arc,
            registry,
            openness,
            width,
            scale,
        );
    });
    collapse.store(ui.ctx());
    drop(module);
    if reset_requested {
        crate::state::client().modules.reset_module_settings(id);
    }
    if reset_requested || favorite_changed {
        crate::config::save();
    }
}

 
const ROW_SWEEP_TIME: f32 = 0.55;

 
 
 
fn paint_row_sweep(ui: &Ui, rect: Rect, name: &str, tint: Color32) {
    let now = ui.input(|i| i.time);
    let Some((start, armed)) =
        ui.data(|data| data.get_temp::<(f64, bool)>(Id::new("row_sweep").with(name)))
    else {
        return;
    };
    let elapsed = (now - start) as f32 / anim::speed();
    if elapsed >= ROW_SWEEP_TIME || elapsed < 0.0 {
        return;
    }
    let phase = (elapsed / ROW_SWEEP_TIME).clamp(0.0, 1.0);
    let strength = if armed { 0.30 } else { 0.14 } * (1.0 - phase).powf(1.6);
    let color = if armed { tint } else { theme::TEXT_MUTED };
    ambience::row_sweep(ui.painter(), rect, phase, color, strength);
    ui.ctx().request_repaint();
}

 
fn paint_reset_icon(ui: &Ui, center: Pos2, scale: f32, hovered: bool, name: &str) {
    const SPIN_TIME: f64 = 0.62;
    let now = ui.input(|input| input.time);
    let spin = ui.data(|data| data.get_temp::<f64>(Id::new("module_reset_spin").with(name)));
    let angle = spin
        .filter(|started| now - *started < SPIN_TIME)
        .map_or(0.0, |started| {
            ui.ctx().request_repaint();
            -((now - started) / SPIN_TIME).clamp(0.0, 1.0) as f32 * std::f32::consts::TAU
        });
    let color = theme::with_alpha(
        if hovered {
            theme::TEXT
        } else {
            theme::TEXT_MUTED
        },
        if hovered { 0.9 } else { 0.58 },
    );
    let (sin, cos) = angle.sin_cos();
    let rotate = |point: Vec2| {
        center + Vec2::new(point.x * cos - point.y * sin, point.x * sin + point.y * cos) * scale
    };
    let painter = ui.painter();
    let stroke = Stroke::new(1.2 * scale, color);
    for (y, knob_x) in [(-3.5_f32, -1.5_f32), (0.0, 2.0), (3.5, -0.2)] {
        painter.line_segment(
            [rotate(Vec2::new(-5.0, y)), rotate(Vec2::new(5.0, y))],
            stroke,
        );
        painter.circle_filled(rotate(Vec2::new(knob_x, y)), 1.45 * scale, color);
    }
}

 
fn paint_chevron(painter: &Painter, center: Pos2, open: f32, color: Color32, scale: f32) {
    let s: f32 = 3.6 * scale;
    let angle = open.clamp(0.0, 1.0) * std::f32::consts::FRAC_PI_2;
    let (sin, cos) = angle.sin_cos();
    let rotate = |v: Vec2| Vec2::new(v.x * cos - v.y * sin, v.x * sin + v.y * cos);
    let points = [
        Vec2::new(s, 0.0),
        Vec2::new(-s * 0.7, -s),
        Vec2::new(-s * 0.7, s),
    ]
    .into_iter()
    .map(|v| center + rotate(v))
    .collect::<Vec<_>>();
    painter.add(Shape::convex_polygon(points, color, Stroke::NONE));
}

 
fn setting_visible(data: &ModuleData, name: &str) -> bool {
    let choice = |name: &str, default| match data.get_setting(name) {
        Some(ModuleSetting::Choice { value, .. }) => *value,
        _ => default,
    };
     
     
    let glow_on = chams_warped::is_warped(data);
    match (data.id, name) {
        (ModuleId::Atmosphere, name) => {
            use crate::graphic::atmosphere::SkyMode;
            let mode = SkyMode::from_choice(choice("Sky", 0));
            match name {
                "Custom Sky" => mode == SkyMode::Custom,
                "Star Density" | "Star Size" | "Twinkle Speed" | "Glow" => {
                    matches!(mode, SkyMode::StarryNight | SkyMode::BlackHole)
                }
                "Black Hole Size" | "Spin Speed" => mode == SkyMode::BlackHole,
                "Flow Speed" if mode == SkyMode::StarfallRain => true,
                "Noise Scale" | "Flow Speed" => matches!(
                    mode,
                    SkyMode::IceNebula
                        | SkyMode::CrystalLattice
                        | SkyMode::FrostVeins
                        | SkyMode::AuroraVeil
                        | SkyMode::EmberDrift
                ),
                "Ice Nebula Color" => mode == SkyMode::IceNebula,
                "Crystal Lattice Color" => mode == SkyMode::CrystalLattice,
                "Frost Veins Color" => mode == SkyMode::FrostVeins,
                "Aurora Veil Color" => mode == SkyMode::AuroraVeil,
                "Ember Drift Color" => mode == SkyMode::EmberDrift,
                "Starfall Rain Color" => mode == SkyMode::StarfallRain,
                _ => true,
            }
        }
        (ModuleId::Clock, "Smooth Seconds") => {
            choice("Style", 0) == 0
                && data
                    .get_setting("Seconds")
                    .and_then(ModuleSetting::get_toggle_value)
                    .unwrap_or(true)
        }
        (ModuleId::MiddleClick, name @ ("Custom Command" | "Swap From Inventory")) => {
            use crate::module::player::middleclick::MiddleAction;
            [("Action", 2), ("Shift Action", 3), ("Ctrl Action", 0)]
                .into_iter()
                .map(|(setting, default)| MiddleAction::from_index(choice(setting, default)))
                .any(|action| match name {
                    "Custom Command" => action == MiddleAction::CustomCommand,
                    _ => matches!(
                        action,
                        MiddleAction::ThrowPearl
                            | MiddleAction::GoldenApple
                            | MiddleAction::Firework
                            | MiddleAction::ChorusFruit
                    ),
                })
        }
         
        (ModuleId::Chams, "Video File" | "Frame Rate") => {
            choice("Style", 0) as i32 == chams_video::STYLE
        }
         
         
        (ModuleId::Chams, name) if chams_warped::ROWS.contains(&name) => glow_on,
        _ => true,
    }
}

 
 
fn sync_chams_clips(data: &mut ModuleData, select: Option<&std::path::Path>) {
    let files = chams_video::clip_files(&chams_video::folder());
    let names = chams_video::clip_names(&files);
    let select = select.and_then(|path| files.iter().position(|file| file == path));
    apply_clip_names(data, &names, select);
}

 
 
fn apply_clip_names(data: &mut ModuleData, names: &[String], select: Option<usize>) {
    let Some(ModuleSetting::Choice { value, options, .. }) =
        data.get_setting_mut(CHAMS_CLIP_SETTING)
    else {
        return;
    };
     
     
    let wanted = select
        .and_then(|index| names.get(index).cloned())
        .or_else(|| options.get(*value).cloned());
    if *options != names {
        *options = names.to_vec();
    }
    *value = wanted
        .and_then(|label| names.iter().position(|name| *name == label))
        .unwrap_or(0);
}

 
 
 
fn refresh_chams_clips(ui: &mut Ui, data: &mut ModuleData) {
    let now = ui.input(|input| input.time);
    let id = Id::new("chams_clip_scan");
    let last: f64 = ui
        .data(|data| data.get_temp(id))
        .unwrap_or(f64::NEG_INFINITY);
    if now - last < 1.0 {
        return;
    }
    ui.data_mut(|data| data.insert_temp(id, now));
    sync_chams_clips(data, None);
}

 
fn draw_settings(
    ui: &mut Ui,
    data: &mut ModuleData,
    arc: &ModuleArc,
    registry: &ModuleMap,
    openness: f32,
    width: f32,
    scale: f32,
) -> bool {
    let mut reset_requested = false;
    let body = |ui: &mut Ui| {
         
         
         
         
        const MEASURE: f32 = 520.0;
        let inner = (width - 20.0 * scale)
            .min(MEASURE * scale)
            .max(60.0 * scale);
        ui.set_min_width(inner);
        ui.set_max_width(inner);
        ui.spacing_mut().item_spacing.y = 7.0 * scale;

        reset_requested = draw_settings_reset_row(ui, data.name(), scale);

         
         
         
         
         
         
        let panel_opacity = ui.opacity();
        if data.id != ModuleId::CustomUi {
            ui.set_opacity(panel_opacity * setting_reveal(openness, 0));
            keybind_row(ui, data, arc, registry, scale);
            ui.set_opacity(panel_opacity);
        }

         
         
         
        let module_name = data.name();
        let mut chams_depth_change = None;
        let mut upload_clip = false;
         
         
         
        if data.id == ModuleId::Chams {
            refresh_chams_clips(ui, data);
        }
        let mut visible_index = 0;
        for index in 0..data.settings.len() {
            if !setting_visible(data, data.settings[index].name()) {
                continue;
            }
            visible_index += 1;
            let setting = &mut data.settings[index];
            let panel_opacity = ui.opacity();
            ui.set_opacity(panel_opacity * setting_reveal(openness, visible_index));
            let setting_name = setting.name().to_owned();
            let help = crate::graphic::tooltips::setting_help(data.id, &setting_name, setting);
             
             
             
            let inner = ui.push_id((module_name, &setting_name), |ui| {
                match setting {
                    ModuleSetting::Toggle { name, value } => {
                        let toggled = controls::switch_row(ui, name, value, scale);
                        if toggled && data.id == ModuleId::Chams {
                            if name == "ThroughWalls" {
                                chams_depth_change = Some(*value);
                            }
                            if name == "OnlyVisible" {
                                chams_depth_change = Some(!*value);
                            }
                        }
                    }
                    ModuleSetting::Slider {
                        name,
                        value,
                        min,
                        max,
                    } => {
                        controls::slider_row(ui, name, value, *min, *max, scale);
                    }
                    ModuleSetting::Choice {
                        name,
                        value,
                        options,
                    } if options.len() <= controls::MAX_PILLS => {
                        controls::choice_row(ui, name, value, options, scale);
                    }
                    ModuleSetting::Choice {
                        name,
                        value,
                        options,
                    } => {
                        ui.label(label(name, scale));
                        egui::ComboBox::from_id_salt(ui.id().with("choice").with(name.as_str()))
                            .width(ui.available_width())
                            .selected_text(
                                RichText::new(
                                    options.get(*value).map(String::as_str).unwrap_or("—"),
                                )
                                .size(12.0 * scale),
                            )
                            .show_ui(ui, |ui| {
                                for (idx, option) in options.iter().enumerate() {
                                    ui.selectable_value(value, idx, option.as_str());
                                }
                            });
                    }
                    ModuleSetting::Color { name, value } => {
                        ui.horizontal(|ui| {
                            ui.label(label(name, scale));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                 
                                 
                                 
                                let mut rgba = egui::Rgba::from_rgba_unmultiplied(
                                    value[0], value[1], value[2], 1.0,
                                );
                                let changed = egui::color_picker::color_edit_button_rgba(
                                    ui,
                                    &mut rgba,
                                    egui::color_picker::Alpha::Opaque,
                                )
                                .changed();
                                if changed {
                                    let [r, g, b, _] = rgba.to_array();
                                    value[0] = r;
                                    value[1] = g;
                                    value[2] = b;
                                }
                            });
                        });
                    }
                    ModuleSetting::BlockList { name, value } => {
                        block_list_row(ui, data.id, module_name, name, value);
                    }
                    ModuleSetting::Text { name, value } => {
                        ui.label(label(name, scale));
                        let field = ui.add(
                            egui::TextEdit::singleline(value).desired_width(ui.available_width()),
                        );
                        focus_glow(ui, &field);
                    }
                }
                 
                if data.id == ModuleId::Chams && setting_name == CHAMS_CLIP_SETTING {
                    upload_clip = controls::action_row(ui, "Upload Video", scale);
                }
            });
             
             
             
            let mut hover_rect = inner.response.rect;
            hover_rect.min.x = ui.max_rect().left();
            hover_rect.max.x = ui.max_rect().right();
            crate::graphic::tooltips::delayed_tooltip(
                ui.ctx(),
                ui.id()
                    .with("setting_tip")
                    .with(module_name)
                    .with(&setting_name),
                inner.response.layer_id,
                ui.rect_contains_pointer(hover_rect),
                &setting_name,
                &help,
            );
            ui.set_opacity(panel_opacity);
        }
        if upload_clip {
            match chams_video::pick() {
                Ok(Some(path)) => {
                     
                     
                    let name = chams_video::clip_name(&path);
                    sync_chams_clips(data, Some(&path));
                    Notification::send(
                        NotificationType::Info,
                        "Chams video",
                        &format!("{name} added to the clip list"),
                    );
                }
                Ok(None) => {}
                Err(error) => Notification::send(
                    NotificationType::Alert,
                    "Chams video",
                    &format!("{error:#}"),
                ),
            }
        }
        if let Some(through) = chams_depth_change {
            if let Some(setting) = data.get_setting_mut("ThroughWalls") {
                setting.set_toggle_value(through);
            }
            if let Some(setting) = data.get_setting_mut("OnlyVisible") {
                setting.set_toggle_value(!through);
            }
        }
        ui.add_space(1.0);
    };
    egui::Frame::none()
        .fill(theme::SURFACE)
        .inner_margin(Margin::symmetric(10.0 * scale, 8.0 * scale))
        .show(ui, body);
    reset_requested
}

 
 
fn draw_settings_reset_row(ui: &mut Ui, name: &str, scale: f32) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Module settings")
                .size(11.0 * scale)
                .color(theme::TEXT_MUTED),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let (rect, response) =
                ui.allocate_exact_size(Vec2::splat(24.0 * scale), Sense::click());
            let center = rect.center();
            if response.clicked() {
                let now = ui.input(|input| input.time);
                ui.data_mut(|data| data.insert_temp(Id::new("module_reset_spin").with(name), now));
                clicked = true;
            }
            paint_reset_icon(ui, center, scale, response.hovered(), name);
            crate::graphic::tooltips::delayed_tooltip(
                ui.ctx(),
                ui.id().with("module_reset_tip").with(name),
                response.layer_id,
                response.hovered(),
                "Reset module settings",
                "Restore this module's factory settings and disable it.",
            );
        });
    });
    clicked
}

 
pub(crate) fn keybind_row(
    ui: &mut Ui,
    data: &mut ModuleData,
    arc: &ModuleArc,
    registry: &ModuleMap,
    scale: f32,
) {
    let row = ui.horizontal(|ui| {
        ui.label(label("Bind", scale));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let bind_id = ui.id().with("kb_listen").with(data.name());
            let listening = ui.data(|d| d.get_temp::<bool>(bind_id).unwrap_or(false));

            let caption = if listening {
                if capture_keybind(data, arc, registry) {
                    ui.data_mut(|d| d.insert_temp(bind_id, false));
                }
                "press…".to_string()
            } else {
                data.key_bind.to_string()
            };

            let color = if listening {
                theme::accent()
            } else {
                theme::TEXT_DIM
            };
             
             
            let listen = anim::toggle(
                ui.ctx(),
                bind_id.with("listen_glow"),
                listening,
                0.2,
                Easing::InOut,
            );
            if listening {
                ui.ctx().request_repaint();
            }
            let fill = theme::lerp_color(
                theme::ELEVATED,
                theme::lerp_color(theme::ELEVATED, theme::accent(), 0.2),
                listen,
            );
            let pulse = 0.5 + 0.5 * (ui.input(|i| i.time) as f32 * 6.0).sin();
            let stroke = if listen > 0.001 {
                Stroke::new(
                    1.0_f32,
                    theme::with_alpha(theme::accent(), listen * (0.4 + 0.4 * pulse)),
                )
            } else {
                Stroke::NONE
            };
            let button = Button::new(RichText::new(caption).size(12.0 * scale).color(color))
                .fill(fill)
                .stroke(stroke);
            if ui.add(button).clicked() {
                let next = !listening;
                ui.data_mut(|d| d.insert_temp(bind_id, next));
                if next {
                    LAST_KEY_PRESSED.store(-1, Ordering::Relaxed);
                }
            }
        });
    });
    crate::graphic::tooltips::delayed_tooltip(
        ui.ctx(),
        ui.id().with("bind_tip").with(data.name()),
        row.response.layer_id,
        ui.rect_contains_pointer(row.response.rect),
        "Bind",
        "Click, then press a key to bind this module. Escape unbinds.",
    );
}

 
 
 
 
fn capture_keybind(data: &mut ModuleData, arc: &ModuleArc, registry: &ModuleMap) -> bool {
    let pressed = LAST_KEY_PRESSED.swap(-1, Ordering::Relaxed);
    if pressed == -1 {
        return false;
    }
    let key = KeyboardKey::from(pressed);

    if key == KeyboardKey::KeyEscape {
        if data.id == ModuleId::MenuHud {
            Notification::send(
                NotificationType::Warning,
                "Menu HUD cannot be unbound",
                "Choose another key instead of Escape to change it",
            );
        } else {
            data.key_bind = KeyboardKey::KeyNone;
        }
        return true;
    }

     
    for other in registry.values() {
        if Arc::ptr_eq(arc, other) {
            continue;
        }
        let owner = other.lock().unwrap();
        if owner.get_module_data().key_bind == key {
            let owner_name = owner.get_module_data().name().to_string();
            drop(owner);
            Notification::send(
                NotificationType::Warning,
                "Keybind in use",
                &format!("'{owner_name}' is bound to {key}"),
            );
            return true;
        }
    }

    data.key_bind = key;
    true
}

 
fn label(text: &str, scale: f32) -> RichText {
    RichText::new(text)
        .size(12.0 * scale)
        .color(theme::TEXT_DIM)
}

 
const RIPPLE_SECS: f32 = 0.45;

 
 
fn setting_reveal(openness: f32, index: usize) -> f32 {
    const STAGGER: f32 = 0.09;
    let delay = (index as f32 * STAGGER).min(0.8);
    Easing::Out.apply(((openness - delay) / (1.0 - delay)).clamp(0.0, 1.0))
}

 
 
 
fn click_ripple(ui: &mut Ui, rect: Rect, response: &egui::Response) {
    let key = response.id.with("ripple");
    let now = ui.ctx().input(|i| i.time);
    if response.clicked() || response.secondary_clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            ui.data_mut(|d| d.insert_temp(key, (pos, now)));
        }
    }
    let Some((origin, start)) = ui.data(|d| d.get_temp::<(Pos2, f64)>(key)) else {
        return;
    };
    let t = ((now - start) as f32 * anim::speed() / RIPPLE_SECS).clamp(0.0, 1.0);
    if t >= 1.0 {
        ui.data_mut(|d| d.remove::<(Pos2, f64)>(key));
        return;
    }
     
    let reach = [
        rect.left_top(),
        rect.right_top(),
        rect.left_bottom(),
        rect.right_bottom(),
    ]
    .into_iter()
    .map(|corner| corner.distance(origin))
    .fold(0.0, f32::max);
    ui.painter().with_clip_rect(rect).circle_filled(
        origin,
        reach * Easing::Out.apply(t),
        theme::with_alpha(theme::accent(), 0.28 * (1.0 - t)),
    );
    ui.ctx().request_repaint();
}

 
fn focus_glow(ui: &mut Ui, response: &egui::Response) {
    let t = anim::toggle(
        ui.ctx(),
        response.id.with("focus_glow"),
        response.has_focus(),
        0.18,
        Easing::Out,
    );
    if t > 0.001 {
        ui.painter().rect_stroke(
            response.rect,
            Rounding::same(theme::RADIUS_INNER),
            Stroke::new(1.0_f32, theme::with_alpha(theme::accent(), t)),
        );
    }
}

 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
fn block_list_row(
    ui: &mut Ui,
    id: ModuleId,
    module_name: &str,
    name: &str,
    value: &mut Vec<String>,
) {
    let storage_only = name == "Storage Blocks";
     
     
     
    let ores_only = id == ModuleId::Xray;
    if storage_only {
         
         
        value.retain(|id| STORAGE_BLOCK_IDS.contains(&id.as_str()));
    } else if ores_only {
         
         
        value.retain(|id| is_ore_block(id));
    }
    let state_id = ui.id().with("blocklist").with(module_name).with(name);
    let mut open = ui
        .data(|d| d.get_temp::<bool>(state_id.with("open")))
        .unwrap_or(false);
    let mut search = ui
        .data(|d| d.get_temp::<String>(state_id.with("search")))
        .unwrap_or_default();
    let default_category = if storage_only {
        "Storage"
    } else if ores_only {
        "Ores"
    } else {
        "All"
    };
    let mut category: String = ui
        .data(|d| d.get_temp::<String>(state_id.with("category")))
        .unwrap_or_else(|| default_category.to_string());
    if storage_only && category != "Storage" && category != "Selected" {
        category = "Storage".to_string();
    }
    if ores_only && category != "Ores" && category != "Selected" {
        category = "Ores".to_string();
    }

     
    ui.horizontal(|ui| {
        let caption = format!("{name} ({})", value.len());
        let color = if open {
            theme::accent()
        } else {
            theme::TEXT_DIM
        };
        let button = Button::new(RichText::new(caption).size(12.0).color(color))
            .fill(theme::ELEVATED)
            .stroke(Stroke::NONE);
        if ui.add(button).clicked() {
            open = !open;
        }
    });
    ui.data_mut(|d| d.insert_temp(state_id.with("open"), open));
    if !open {
        return;
    }

    let search_field = egui::TextEdit::singleline(&mut search)
        .hint_text("Search blocks…")
        .desired_width(ui.available_width());
    let field = ui.add(search_field);
    focus_glow(ui, &field);
    ui.data_mut(|d| d.insert_temp(state_id.with("search"), search.clone()));

     
     
    let chips = if storage_only {
        vec!["Storage".to_string(), "Selected".to_string()]
    } else if ores_only {
        vec!["Ores".to_string(), "Selected".to_string()]
    } else {
        let mut chips = vec!["All".to_string()];
        chips.extend(BLOCK_GROUPS.iter().map(|group| group.label.to_string()));
        chips.push("Selected".to_string());
        chips.push("Custom".to_string());
        chips
    };
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for chip in &chips {
            let selected = *chip == category;
            let text = RichText::new(chip.as_str()).size(11.5).color(if selected {
                Color32::BLACK
            } else {
                theme::TEXT_DIM
            });
            let fill = if selected {
                theme::accent()
            } else {
                theme::ELEVATED
            };
            if ui
                .add(Button::new(text).fill(fill).stroke(Stroke::NONE))
                .clicked()
            {
                category = chip.clone();
            }
        }
    });
    ui.data_mut(|d| d.insert_temp(state_id.with("category"), category.clone()));

     
     
    let needle = search.to_lowercase();
    let rows: Vec<String> = if category == "Selected" {
        value.clone()
    } else if storage_only && category == "Storage" {
        STORAGE_BLOCK_IDS
            .iter()
            .map(|id| (*id).to_string())
            .collect()
    } else if ores_only {
        ORE_BLOCK_IDS.iter().map(|id| (*id).to_string()).collect()
    } else if category == "Custom" {
        value
            .iter()
            .filter(|id| !is_known_block(id))
            .cloned()
            .collect()
    } else if category == "All" {
        all_blocks().to_vec()
    } else {
        BLOCK_GROUPS
            .iter()
            .find(|group| group.label == category)
            .map(|group| group.ids.iter().map(|id| id.to_string()).collect())
            .unwrap_or_default()
    };
    let rows: Vec<String> = rows
        .into_iter()
        .filter(|id| {
            needle.is_empty()
                || id.contains(&needle)
                || display_name(id).to_lowercase().contains(&needle)
        })
        .collect();

    let mut scroll = egui::style::ScrollStyle::floating();
    scroll.bar_width = 5.0;
    ui.style_mut().spacing.scroll = scroll;
     
     
    egui::ScrollArea::vertical()
        .id_salt(state_id.with("list"))
        .max_height(150.0)
        .auto_shrink([false, true])
        .show_rows(ui, 18.0, rows.len(), |ui, range| {
            ui.set_min_width(PANEL_W - 20.0);
            if rows.is_empty() {
                ui.label(
                    RichText::new(match category.as_str() {
                        "Custom" => "No custom blocks — add one below.",
                        "Selected" => "Nothing selected yet.",
                        _ => "No blocks match.",
                    })
                    .size(11.5)
                    .color(theme::TEXT_MUTED),
                );
            }
            for id in rows[range].to_vec() {
                let selected = value.contains(&id);
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 18.0), Sense::click());
                let painter = ui.painter_at(rect);
                if selected {
                    painter.rect_filled(
                        rect,
                        Rounding::same(3.0),
                        theme::with_alpha(
                            theme::accent(),
                            if response.hovered() { 0.32 } else { 0.18 },
                        ),
                    );
                } else if response.hovered() {
                    painter.rect_filled(rect, Rounding::same(3.0), theme::SURFACE_HOVER);
                }
                painter.text(
                    pos2(rect.min.x + 6.0, rect.center().y),
                    Align2::LEFT_CENTER,
                    display_name(&id),
                    FontId::proportional(11.5),
                    if selected {
                        theme::TEXT
                    } else {
                        theme::TEXT_DIM
                    },
                );
                 
                let square = Rect::from_center_size(
                    pos2(rect.max.x - 12.0, rect.center().y),
                    Vec2::splat(11.0),
                );
                if selected {
                    painter.rect_filled(square, Rounding::same(2.5), theme::accent());
                } else {
                    painter.rect_filled(
                        square,
                        Rounding::same(2.5),
                        Color32::from_black_alpha(120),
                    );
                    painter.rect_stroke(
                        square,
                        Rounding::same(2.5),
                        Stroke::new(1.0_f32, theme::BORDER),
                    );
                }
                if response.clicked() {
                    if selected {
                        value.retain(|existing| existing != &id);
                    } else {
                        value.push(id.clone());
                    }
                }
            }
        });

     
     
     
    if !storage_only && !ores_only {
        ui.horizontal(|ui| {
            let mut add = ui
                .data(|d| d.get_temp::<String>(state_id.with("add")))
                .unwrap_or_default();
            let field = egui::TextEdit::singleline(&mut add)
                .hint_text("add: minecraft:sculk_sensor")
                .desired_width(ui.available_width() - 30.0);
            let response = ui.add(field);
            focus_glow(ui, &response);
            let submitted = ui
                .add(
                    Button::new(RichText::new("+").size(12.0).color(theme::TEXT_DIM))
                        .fill(theme::ELEVATED)
                        .stroke(Stroke::NONE),
                )
                .clicked()
                || (response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)));
            if submitted {
                if let Some(id) = normalize_id(&add) {
                    if !value.contains(&id) {
                        value.push(id);
                    }
                    add.clear();
                }
            }
            ui.data_mut(|d| d.insert_temp(state_id.with("add"), add));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphic::preview::{gather_textures, rasterize};
    use crate::module::Module;

    fn set_choice(data: &mut ModuleData, name: &str, selected: usize) {
        match data.get_setting_mut(name).unwrap() {
            ModuleSetting::Choice { value, .. } => *value = selected,
            _ => panic!("{name} is not a choice"),
        }
    }

    #[test]
    fn atmosphere_only_shows_controls_for_the_selected_sky() {
        let mut data = ModuleData {
            id: ModuleId::Atmosphere,
            category: ModuleCategory::Visuals,
            description: String::new(),
            key_bind: KeyboardKey::KeyNone,
            enabled: false,
            settings: vec![ModuleSetting::Choice {
                name: "Sky".into(),
                value: 0,
                options: vec![],
            }],
        };
        for mode in 0..6 {
            set_choice(&mut data, "Sky", mode);
            for name in ["Star Density", "Star Size", "Twinkle Speed", "Glow"] {
                assert_eq!(
                    setting_visible(&data, name),
                    mode < 2,
                    "{name}, mode {mode}"
                );
            }
            for name in ["Black Hole Size", "Spin Speed"] {
                assert_eq!(
                    setting_visible(&data, name),
                    mode == 1,
                    "{name}, mode {mode}"
                );
            }
            for name in ["Noise Scale", "Flow Speed"] {
                assert_eq!(
                    setting_visible(&data, name),
                    mode >= 3,
                    "{name}, mode {mode}"
                );
            }
            for (name, scene) in [
                ("Ice Nebula Color", 3),
                ("Crystal Lattice Color", 4),
                ("Frost Veins Color", 5),
                ("Aurora Veil Color", 6),
                ("Ember Drift Color", 7),
            ] {
                assert_eq!(
                    setting_visible(&data, name),
                    mode == scene,
                    "{name}, mode {mode}"
                );
            }
            assert_eq!(setting_visible(&data, "Custom Sky"), mode == 2);
            assert!(setting_visible(&data, "Sky"));
            assert!(setting_visible(&data, "Brightness"));
        }
        set_choice(&mut data, "Sky", 99);
        assert!(setting_visible(&data, "Star Size"));
        assert!(!setting_visible(&data, "Custom Sky"));
    }

     
     
    #[test]
    fn chams_clip_controls_follow_the_video_look_and_the_folder() {
        use crate::module::render::chams::{ChamsModule, CLIP_SETTING};
        let mut module = ChamsModule::new();
        let data = &mut module.module;
        assert!(
            !setting_visible(data, CLIP_SETTING),
            "Neon Trail has no clip"
        );
        set_choice(data, "Style", chams_video::STYLE as usize);
        assert!(setting_visible(data, CLIP_SETTING));
        assert!(setting_visible(data, "Frame Rate"));

         
        apply_clip_names(data, &["bear".into(), "cat".into(), "dog".into()], None);
        set_choice(data, CLIP_SETTING, 2);
        apply_clip_names(
            data,
            &["bear".into(), "cat".into(), "dog".into(), "eel".into()],
            None,
        );
        let label = |data: &ModuleData| match data.get_setting(CLIP_SETTING) {
            Some(ModuleSetting::Choice { value, options, .. }) => options[*value].clone(),
            _ => panic!("no clip choice"),
        };
        assert_eq!(label(data), "dog");

         
         
        apply_clip_names(data, &["bear".into(), "cat".into(), "dog".into()], Some(1));
        assert_eq!(label(data), "cat");
        apply_clip_names(data, &["bear".into(), "dog".into()], None);
        assert_eq!(label(data), "bear");
    }

     
     
    #[test]
    fn chams_rows_follow_the_selected_style() {
        use crate::module::render::chams::{ChamsModule, STYLES};
        let mut module = ChamsModule::new();
        let data = &mut module.module;
         
        assert_eq!(chams_warped::style_index(data), 0);
        for name in chams_warped::ROWS {
            assert!(
                setting_visible(data, name),
                "{name} is available to classic looks"
            );
        }
        assert!(setting_visible(data, "Outline Glow"));
        for name in ["Style", "GlowStrength", "Pulse"] {
            assert!(
                setting_visible(data, name),
                "{name} is the classical look's row"
            );
        }
        for name in ["Video File", "Frame Rate"] {
            assert!(!setting_visible(data, name), "{name} needs the Video look");
        }

         
        set_choice(data, "Style", chams_warped::FIRST_STYLE);
        for name in chams_warped::ROWS {
            assert!(
                setting_visible(data, name),
                "{name} is shown for a Warped look"
            );
        }
         
        set_choice(data, "Style", 0);
        for name in chams_warped::ROWS {
            assert!(setting_visible(data, name), "{name} remains available");
        }
        if let Some(ModuleSetting::Toggle { value, .. }) = data.get_setting_mut("Outline Glow") {
            *value = false;
        }
        for name in chams_warped::ROWS {
            assert!(
                !setting_visible(data, name),
                "{name} hides when Outline Glow is off"
            );
        }
        assert!(setting_visible(data, "Outline Glow"));

         
        assert_eq!(STYLES.len(), chams_warped::LAST_STYLE + 1);
        assert_eq!(
            STYLES[chams_warped::FIRST_STYLE],
            chams_warped::MODE_NAMES[0]
        );
        assert_eq!(
            STYLES[chams_warped::LAST_STYLE],
            chams_warped::MODE_NAMES[chams_warped::MODE_COUNT - 1]
        );
         
        for name in [
            "Players",
            "Self",
            "Armor",
            "Color",
            "ThroughWalls",
            "OnlyVisible",
            "Range",
        ] {
            assert!(setting_visible(data, name), "{name} still applies");
        }
    }

    #[test]
    fn middle_click_settings_follow_all_three_actions() {
        use crate::module::player::middleclick::{MiddleAction, MiddleClickModule};
        let mut module = MiddleClickModule::new();
        let data = &mut module.module;
        assert!(!setting_visible(data, "Custom Command"));
        assert!(setting_visible(data, "Swap From Inventory"));
        for name in ["Action", "Shift Action", "Ctrl Action"] {
            set_choice(data, name, MiddleAction::None.index());
        }
        assert!(!setting_visible(data, "Swap From Inventory"));
        for name in ["Action", "Shift Action", "Ctrl Action"] {
            set_choice(data, name, MiddleAction::CustomCommand.index());
            assert!(setting_visible(data, "Custom Command"), "{name}");
            set_choice(data, name, MiddleAction::None.index());
        }
        for action in [
            MiddleAction::ThrowPearl,
            MiddleAction::GoldenApple,
            MiddleAction::Firework,
            MiddleAction::ChorusFruit,
        ] {
            set_choice(data, "Ctrl Action", action.index());
            assert!(setting_visible(data, "Swap From Inventory"), "{action:?}");
        }
        set_choice(data, "Ctrl Action", MiddleAction::Refill.index());
        assert!(!setting_visible(data, "Swap From Inventory"));
    }

    #[test]
    fn clock_smooth_seconds_follows_face_and_seconds_without_losing_values() {
        let mut module = crate::module::render::legit::clock();
        let data = &mut module.module;
        assert!(setting_visible(data, "Smooth Seconds"));
        data.get_setting_mut("Smooth Seconds")
            .unwrap()
            .set_toggle_value(false);
        data.get_setting_mut("Seconds")
            .unwrap()
            .set_toggle_value(false);
        assert!(!setting_visible(data, "Smooth Seconds"));
        data.get_setting_mut("Seconds")
            .unwrap()
            .set_toggle_value(true);
        assert!(setting_visible(data, "Smooth Seconds"));
        set_choice(data, "Style", 1);
        assert!(!setting_visible(data, "Smooth Seconds"));
        set_choice(data, "Style", 0);
        assert!(setting_visible(data, "Smooth Seconds"));
        assert_eq!(
            data.get_setting("Smooth Seconds")
                .unwrap()
                .get_toggle_value(),
            Some(false)
        );
    }

    #[test]
    fn expanded_atmosphere_panel_switches_settings_and_keeps_the_selection() {
        use egui::RawInput;

        let module = crate::module::render::legit::clock();
        let arc: ModuleArc = Arc::new(Mutex::new(Box::new(module)));
        let registry = ModuleMap::new();
        let mut data = ModuleData {
            id: ModuleId::Atmosphere,
            category: ModuleCategory::Visuals,
            description: String::new(),
            key_bind: KeyboardKey::KeyNone,
            enabled: false,
            settings: vec![
                ModuleSetting::Choice {
                    name: "Sky".into(),
                    value: 2,
                    options: [
                        "Starry Night",
                        "Black Hole",
                        "Custom Image",
                        "Ice Nebula",
                        "Crystal Lattice",
                        "Frost Veins",
                        "Aurora Veil",
                        "Ember Drift",
                    ]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                },
                ModuleSetting::Choice {
                    name: "Custom Sky".into(),
                    value: 1,
                    options: ["First", "Second"].into_iter().map(str::to_owned).collect(),
                },
                ModuleSetting::Slider {
                    name: "Star Size".into(),
                    value: 1.0,
                    min: 0.4,
                    max: 2.5,
                },
            ],
        };
        let ctx = Context::default();
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(500.0, 800.0));
        for (frame, mode) in [2, 0, 2].into_iter().enumerate() {
            set_choice(&mut data, "Sky", mode);
            let output = ctx.run(
                RawInput {
                    screen_rect: Some(screen),
                    time: Some(frame as f64 / 60.0),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        draw_settings(ui, &mut data, &arc, &registry, 1.0, 360.0, 1.0);
                    });
                },
            );
            let mut labels = Vec::new();
            for shape in &output.shapes {
                labels_in(&shape.shape, &mut labels);
            }
            assert_eq!(labels.iter().any(|text| text == "Custom Sky"), mode == 2);
            assert_eq!(labels.iter().any(|text| text == "Star Size"), mode == 0);
            assert!(matches!(
                data.get_setting("Custom Sky"),
                Some(ModuleSetting::Choice { value: 1, .. })
            ));
        }
    }

    #[test]
    fn expanded_clock_panel_updates_its_visible_rows() {
        use egui::{Event, PointerButton, RawInput};

        let module = crate::module::render::legit::clock();
        let mut data = module.get_module_data().clone();
        let arc: ModuleArc = Arc::new(Mutex::new(Box::new(module)));
        let registry = ModuleMap::new();
        let ctx = Context::default();
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(500.0, 800.0));
        let mut frame = 0;
        let mut draw = |events| {
            frame += 1;
            ctx.run(
                RawInput {
                    screen_rect: Some(screen),
                    time: Some(frame as f64 / 60.0),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        draw_settings(ui, &mut data, &arc, &registry, 1.0, 360.0, 1.0);
                    });
                },
            )
        };
        let output = draw(vec![]);
        let mut labels = Vec::new();
        for shape in &output.shapes {
            labels_in(&shape.shape, &mut labels);
        }
        assert!(labels.iter().any(|text| text == "Smooth Seconds"));
        let seconds = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                Shape::Text(text) if text.galley.job.text == "Seconds" => {
                    Some(text.pos + egui::vec2(10.0, 0.0))
                }
                _ => None,
            })
            .expect("seconds switch is painted");
        draw(vec![
            Event::PointerMoved(seconds),
            Event::PointerButton {
                pos: seconds,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            },
        ]);
        let output = draw(vec![Event::PointerButton {
            pos: seconds,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        }]);
        let mut labels = Vec::new();
        for shape in &output.shapes {
            labels_in(&shape.shape, &mut labels);
        }
        assert!(!labels.iter().any(|text| text == "Smooth Seconds"));
        draw(vec![
            Event::PointerMoved(seconds),
            Event::PointerButton {
                pos: seconds,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            },
        ]);
        let output = draw(vec![Event::PointerButton {
            pos: seconds,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        }]);
        let mut labels = Vec::new();
        for shape in &output.shapes {
            labels_in(&shape.shape, &mut labels);
        }
        assert!(labels.iter().any(|text| text == "Smooth Seconds"));
    }

    #[test]
    fn empty_query_matches_nothing() {
        assert!(!search_matches("", "KillAura"));
        assert!(!search_matches("", "Player ESP"));
    }

    #[test]
    fn query_matches_against_the_folded_name() {
         
        assert!(search_matches("aura", "KillAura"));
        assert!(search_matches("esp", "Player ESP"));
        assert!(!search_matches("killaura", "MobAura"));
    }

    #[test]
    fn panel_search_clear_button_removes_query_and_results() {
        use egui::{Event, PointerButton, RawInput};
        let registry = crate::module::registry::ModuleRegistry::new();
        registry.register(crate::module::render::player_esp::PlayerEspModule::new());
        let map = registry.by_id();
        let arc = map.get(&ModuleId::PlayerEsp).unwrap();
        let members = [Member {
            name: ModuleId::PlayerEsp.display_name(),
            arc,
        }];
        let ctx = Context::default();
        let query_id = Id::new("search_query");
        ctx.data_mut(|data| data.insert_temp(query_id, "esp player".to_owned()));
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
        let draw = |time, events| {
            ctx.run(
                RawInput {
                    screen_rect: Some(screen),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    draw_search_panel(
                        ctx,
                        Chrome::resting(1.0),
                        pos2(20.0, 20.0),
                        &members,
                        &map,
                        0,
                    )
                },
            )
        };
        let _ = draw(1.0, vec![]);
        let output = draw(1.1, vec![]);
        let mut labels = Vec::new();
        for shape in &output.shapes {
            labels_in(&shape.shape, &mut labels);
        }
        assert!(labels.iter().any(|text| text == "Player ESP"));
        let clear = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                Shape::Text(text) if text.galley.job.text == "×" => {
                    Some(text.visual_bounding_rect().center())
                }
                _ => None,
            })
            .expect("clear button is visible");
        let _ = draw(
            1.2,
            vec![
                Event::PointerMoved(clear),
                Event::PointerButton {
                    pos: clear,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
        );
        let output = draw(
            1.3,
            vec![Event::PointerButton {
                pos: clear,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        );
        assert_eq!(
            ctx.data(|data| data.get_temp::<String>(query_id)),
            Some(String::new())
        );
        let mut labels = Vec::new();
        for shape in &output.shapes {
            labels_in(&shape.shape, &mut labels);
        }
        assert!(!labels.iter().any(|text| text == "Player ESP"));
    }

    #[test]
    fn query_can_span_words() {
        assert!(search_matches("er es", "Player ESP"));
        assert!(search_matches("player esp ", "Player ESP"));
        assert!(search_matches("playeresp", "Player ESP"));
        assert!(search_matches("ESP player", "Player ESP"));
        assert!(search_matches("esp   player", "Player ESP"));
        assert!(!search_matches("esp mob", "Player ESP"));
        assert!(!search_matches("   ", "Player ESP"));
    }

     
     
     
    #[test]
    fn the_minimise_chevron_turns_over() {
        let centre = Pos2::new(40.0, 20.0);
        let open = collapse_chevron(centre, 0.0, COLLAPSE_ICON_SIZE);
        let shut = collapse_chevron(centre, 1.0, COLLAPSE_ICON_SIZE);
         
        assert!(open[1].y < open[0].y && open[1].y < open[2].y);
         
         
        for (a, b) in open.iter().zip(shut.iter()) {
            assert!(
                (b.x - (2.0 * centre.x - a.x)).abs() < 1e-4,
                "{a:?} -> {b:?}"
            );
            assert!(
                (b.y - (2.0 * centre.y - a.y)).abs() < 1e-4,
                "{a:?} -> {b:?}"
            );
        }
        assert!(shut[1].y > shut[0].y && shut[1].y > shut[2].y);
    }

     
     
    #[test]
    fn the_chevron_overshoot_only_rotates() {
        let centre = Pos2::new(-8.5, 3.25);
        let arms = [(1.0_f32, 0.58_f32), (0.0, -0.58), (-1.0, 0.58)];
        for turn in [-0.12_f32, 0.0, 0.35, 0.5, 1.0, 1.13] {
            let points = collapse_chevron(centre, turn, COLLAPSE_ICON_SIZE);
            for (point, arm) in points.iter().zip(arms) {
                assert!(point.x.is_finite() && point.y.is_finite());
                let reach = (*point - centre).length();
                let want = (arm.0 * arm.0 + arm.1 * arm.1).sqrt() * COLLAPSE_ICON_SIZE;
                assert!(
                    (reach - want).abs() < 1e-3,
                    "turn {turn}: {reach} != {want}"
                );
            }
        }
    }

     
     
    #[test]
    fn the_accent_line_lengthens_as_a_panel_closes() {
        assert!((accent_line_len(0.0) - PANEL_W * 0.58).abs() < 1e-3);
        assert!((accent_line_len(1.0) - PANEL_W).abs() < 1e-3);
        assert!(accent_line_len(0.5) > accent_line_len(0.2));
        assert_eq!(accent_line_len(4.0), accent_line_len(1.0));
        assert_eq!(accent_line_len(-2.0), accent_line_len(0.0));
    }

    #[test]
    fn panel_reveal_ripples_left_to_right_and_settles() {
        assert_eq!(panel_reveal(0.0, 0), 0.0);
        assert_eq!(panel_reveal(1.0, 0), 1.0);
        assert_eq!(panel_reveal(1.0, 8), 1.0);
        assert!(
            panel_reveal(0.4, 0) > panel_reveal(0.4, 3),
            "earlier panels should be further into their reveal"
        );
    }

    #[test]
    fn open_toolbar_tiles_are_opaque_and_in_place() {
        let tile = toolbar_reveal(1.0, 6);
        assert!((tile.alpha - 1.0).abs() < 1e-3, "alpha {}", tile.alpha);
        assert!(tile.dy.abs() < 1e-3, "dy {}", tile.dy);
    }

    #[test]
    fn closed_toolbar_tiles_are_hidden_above_their_rest() {
        let tile = toolbar_reveal(0.0, 0);
        assert!(tile.alpha < 1e-3, "alpha {}", tile.alpha);
        assert!(
            (tile.dy + TOOLBAR_REVEAL_DROP).abs() < 1e-3,
            "dy {}",
            tile.dy
        );
    }

    #[test]
    fn later_toolbar_tiles_start_after_earlier_ones() {
        assert!(toolbar_reveal(0.3, 0).alpha > toolbar_reveal(0.3, 6).alpha);
    }

     
     
     
     
     
    #[test]
    fn the_title_bar_control_turns_over_on_the_paint_path() {
        use egui::{RawInput, Shape};

         
         
        let category = ModuleCategory::World;
        crate::config::set_collapsed(category, false);

        let ctx = Context::default();
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(640.0, 480.0));
        let paint = |time: f64| -> Vec<Shape> {
            ctx.run(
                RawInput {
                    screen_rect: Some(screen),
                    time: Some(time),
                    predicted_dt: 1.0 / 60.0,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        draw_title_bar(ui, category, Some(4), true);
                    });
                },
            )
            .shapes
            .into_iter()
            .map(|clipped| clipped.shape)
            .collect()
        };
         
         
         
        let discs = |shapes: &[Shape]| -> (Vec<(Pos2, f32)>, Vec<(Pos2, f32)>) {
            let (mut halo, mut caps) = (Vec::new(), Vec::new());
            for shape in shapes {
                let Shape::Circle(circle) = shape else {
                    continue;
                };
                if circle.fill.a() == 0 {
                    continue;
                }
                let disc = (circle.center, circle.radius);
                if circle.radius > 2.0 {
                    halo.push(disc);
                } else {
                    caps.push(disc);
                }
            }
            (halo, caps)
        };
         
         
         
         
         
        let ring = |shapes: &[Shape], centre: Pos2| -> Option<Color32> {
            shapes.iter().find_map(|shape| match shape {
                Shape::Circle(circle)
                    if circle.fill.a() == 0 && (circle.center - centre).length() < 0.5 =>
                {
                    Some(circle.stroke.color)
                }
                _ => None,
            })
        };
        let near = |a: Color32, b: Color32| {
            (a.r() as i32 - b.r() as i32).abs() <= 1
                && (a.g() as i32 - b.g() as i32).abs() <= 1
                && (a.b() as i32 - b.b() as i32).abs() <= 1
        };
        let vertices_are = |caps: &[(Pos2, f32)], want: [Pos2; 3], tolerance: f32| {
            for (point, _) in caps {
                assert!(
                    want.iter().any(|w| (*w - *point).length() <= tolerance),
                    "painted {point:?} is not a vertex of {want:?}"
                );
            }
        };

         
        let open_frame = paint(0.0);
        let (halo, caps) = discs(&open_frame);
        assert_eq!(
            halo.len(),
            3,
            "the soft edge is three nested discs: {halo:?}"
        );
        assert_eq!(
            caps.len(),
            3,
            "each vertex of the chevron is capped: {caps:?}"
        );
        let centre = halo[0].0;
        for (c, _) in &halo {
            assert!(
                (*c - centre).length() < 1e-3,
                "the nested discs share one centre: {halo:?}"
            );
        }
        let mut radii: Vec<f32> = halo.iter().map(|(_, r)| *r).collect();
        radii.sort_by(f32::total_cmp);
        assert!((radii[1] - radii[0] - 1.2).abs() < 1e-3, "{radii:?}");
        assert!((radii[2] - radii[1] - 1.2).abs() < 1e-3, "{radii:?}");
        assert!(
            (radii[0] - COLLAPSE_BTN_RADIUS).abs() < 1e-3,
            "an unhovered control sits at its rest size: {radii:?}"
        );
        let open = collapse_chevron(centre, 0.0, COLLAPSE_ICON_SIZE);
        vertices_are(&caps, open, 1e-2);
        assert!(
            open[1].y < centre.y,
            "an open panel offers a minimise arrow"
        );
        let open_ring = ring(&open_frame, centre).expect("the control is outlined");
        assert!(
            !near(open_ring, theme::accent()),
            "an open panel's ring is not accented yet: {open_ring:?}"
        );

         
        crate::config::set_collapsed(category, true);
        let mut settled = Vec::new();
        for step in 1..=240 {
            settled = paint(step as f64 / 60.0);
        }
        crate::config::set_collapsed(category, false);

        let (halo_shut, caps_shut) = discs(&settled);
        assert_eq!(halo_shut.len(), 3, "{halo_shut:?}");
        assert_eq!(caps_shut.len(), 3, "{caps_shut:?}");
        let centre_shut = halo_shut[0].0;
        assert!(
            (centre_shut - centre).length() < 1e-3,
            "the icon turns in place: {centre:?} -> {centre_shut:?}"
        );
        let shut = collapse_chevron(centre_shut, 1.0, COLLAPSE_ICON_SIZE);
        vertices_are(&caps_shut, shut, 0.05);
        assert!(
            shut[1].y > centre_shut.y,
            "a minimised panel offers a maximise arrow"
        );
        let shut_ring = ring(&settled, centre_shut).expect("the control is outlined");
        assert!(
            near(shut_ring, theme::accent()),
            "a minimised panel's ring is the accent: {shut_ring:?}"
        );
    }

     
    fn render_panel_members(map: &ModuleMap) -> Vec<Member<'_>> {
        let mut members: Vec<Member> = map
            .values()
            .map(|arc| {
                let module = arc.lock().unwrap();
                let data = module.get_module_data();
                Member {
                    name: data.id.display_name(),
                    arc,
                }
            })
            .collect();
        members.sort_by_key(|member| member.name);
        members
    }

     
     
     
    fn render_panel_labels() -> Vec<String> {
        use crate::module::registry::ModuleRegistry;
        use egui::{RawInput, Shape};

        fn collect(shape: &Shape, out: &mut Vec<String>) {
            match shape {
                Shape::Text(text) => out.push(text.galley.job.text.clone()),
                Shape::Vec(shapes) => {
                    for shape in shapes {
                        collect(shape, out);
                    }
                }
                _ => {}
            }
        }

        let registry = ModuleRegistry::new();
        registry.register(crate::module::render::player_esp::PlayerEspModule::new());
        registry.register(crate::module::render::tracers::TracersModule::new());
        registry.register(crate::module::render::hand_aura::HandAuraModule::new());
        registry.register(crate::module::render::smoke::SmokeModule::new());
        registry.register(crate::module::render::legit::coords());
        registry.register(crate::module::render::legit::ping());

        let map = registry.by_id();
        let members = render_panel_members(&map);

        let ctx = Context::default();
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 900.0));
        let mut texts = Vec::new();
         
         
        for frame in 0..4 {
            let output = ctx.run(
                RawInput {
                    screen_rect: Some(screen),
                    time: Some(frame as f64 / 60.0),
                    predicted_dt: 1.0 / 60.0,
                    ..Default::default()
                },
                |ctx| {
                    draw_panel(
                        ctx,
                        Chrome::resting(1.0),
                        ModuleCategory::Render,
                        &members,
                        pos2(20.0, 20.0),
                        &map,
                        0,
                    );
                },
            );
            for clipped in output.shapes {
                collect(&clipped.shape, &mut texts);
            }
        }
        texts
    }

     
     
     
    #[test]
    fn the_render_panel_preview_renders_lit_pixels() {
        use crate::module::registry::ModuleRegistry;

        let registry = ModuleRegistry::new();
        registry.register(crate::module::render::player_esp::PlayerEspModule::new());
        registry.register(crate::module::render::tracers::TracersModule::new());
        registry.register(crate::module::render::hand_aura::HandAuraModule::new());
        registry.register(crate::module::render::smoke::SmokeModule::new());
        registry.register(crate::module::render::legit::coords());
        registry.register(crate::module::render::legit::ping());
         
         
        for id in [ModuleId::PlayerEsp, ModuleId::HandAura, ModuleId::Coords] {
            if let Some(arc) = registry.get(id) {
                arc.lock().unwrap().get_module_data_mut().set_enabled(true);
            }
        }
        let map = registry.by_id();
        let members = render_panel_members(&map);

        let (width, height) = (300usize, 360usize);
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(width as f32, height as f32));
        let ctx = Context::default();
        let mut textures = HashMap::new();
        let mut primitives = Vec::new();
        for frame in 0..4 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(frame as f64 / 60.0),
                    predicted_dt: 1.0 / 60.0,
                    ..Default::default()
                },
                |ctx| {
                    draw_panel(
                        ctx,
                        Chrome::resting(1.0),
                        ModuleCategory::Render,
                        &members,
                        pos2(20.0, 20.0),
                        &map,
                        0,
                    );
                },
            );
            gather_textures(&mut textures, &output.textures_delta);
            primitives = ctx.tessellate(output.shapes, 1.0);
        }
         
        let backdrop = [0.06, 0.06, 0.075, 1.0];
        let image = rasterize(primitives, &textures, width, height, backdrop);
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../target/render-tab-preview.png");
        image.save(&path).expect("the preview png saves");
        let lit = image
            .pixels()
            .filter(|pixel| {
                pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32 > 60 && pixel[3] > 128
            })
            .count();
        println!("render tab preview: {lit} lit pixels -> {path:?}");
        assert!(lit > 4_000, "the render panel preview is nearly empty");
    }

     
     
     
    #[test]
    fn the_clickgui_dims_the_world_behind_it() {
        let ctx = Context::default();
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(640.0, 360.0));
         
        let washes = |progress: f32| -> Vec<Color32> {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ctx| draw_backdrop(ctx, progress),
            );
            output
                .shapes
                .into_iter()
                .filter_map(|clipped| match clipped.shape {
                    egui::Shape::Rect(rect) if rect.rect == screen && rect.fill.a() > 0 => {
                        Some(rect.fill)
                    }
                    _ => None,
                })
                .collect()
        };

        let open = washes(1.0);
        let full = (BACKDROP_DIM * 255.0).round() as u8;
        assert!(
            open.iter()
                .any(|fill| *fill == Color32::from_black_alpha(full)),
            "a fully open menu lays the wash over the whole screen: {open:?}"
        );
         
        let half = washes(0.5);
        let half_alpha = (BACKDROP_DIM * 255.0 * 0.5).round() as u8;
        assert!(
            half.iter()
                .any(|fill| *fill == Color32::from_black_alpha(half_alpha)),
            "a half-open menu has half the wash: {half:?}"
        );
        assert!(
            washes(0.0).is_empty(),
            "a closed menu leaves the world alone"
        );
    }

     
     
    #[test]
    fn the_wash_lifts_as_the_interface_thins() {
        let opaque = backdrop_wash(1.0, 1.0);
        assert_eq!(
            opaque,
            Color32::from_black_alpha((BACKDROP_DIM * 255.0).round() as u8),
            "a solid menu keeps the full wash"
        );
        let thin = backdrop_wash(1.0, 0.6);
        assert!(
            thin.a() < opaque.a(),
            "{thin:?} is not thinner than {opaque:?}"
        );
        assert!(thin.a() > 0, "the menu always sits on some wash");
    }

     
     
     
    #[test]
    #[ignore = "writes a preview into screenshots/"]
    fn clickgui_ambience_preview() {
        use crate::module::registry::ModuleRegistry;

        let registry = ModuleRegistry::new();
        registry.register(crate::module::render::player_esp::PlayerEspModule::new());
        registry.register(crate::module::render::tracers::TracersModule::new());
        registry.register(crate::module::render::hand_aura::HandAuraModule::new());
        registry.register(crate::module::render::smoke::SmokeModule::new());
        registry.register(crate::module::render::legit::coords());
        registry.register(crate::module::combat::killaura::KillAuraModule::new());
        registry.register(crate::module::movement::sprint::SprintModule::new());
        for id in [ModuleId::PlayerEsp, ModuleId::HandAura, ModuleId::Coords] {
            if let Some(arc) = registry.get(id) {
                arc.lock().unwrap().get_module_data_mut().set_enabled(true);
            }
        }
        let map = registry.by_id();
        let render = render_panel_members(&map);
        let combat: Vec<Member> = map
            .iter()
            .filter(|(_, arc)| {
                arc.lock().unwrap().get_module_data().category == ModuleCategory::Combat
            })
            .map(|(id, arc)| Member {
                name: id.display_name(),
                arc,
            })
            .collect();

        let (width, height) = (640usize, 400usize);
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(width as f32, height as f32));
        let ctx = Context::default();
        let mut textures = HashMap::new();
        let mut primitives = Vec::new();
        for frame in 0..5 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(4.0 + frame as f64 / 60.0),
                    predicted_dt: 1.0 / 60.0,
                    events: vec![egui::Event::PointerMoved(pos2(300.0, 120.0))],
                    ..Default::default()
                },
                |ctx| {
                    draw_backdrop(ctx, 1.0);
                    draw_panel(
                        ctx,
                        Chrome::resting(1.0),
                        ModuleCategory::Combat,
                        &combat,
                        pos2(28.0, 48.0),
                        &map,
                        0,
                    );
                    draw_panel(
                        ctx,
                        Chrome::resting(1.0),
                        ModuleCategory::Render,
                        &render,
                        pos2(210.0, 48.0),
                        &map,
                        1,
                    );
                },
            );
            gather_textures(&mut textures, &output.textures_delta);
            primitives = ctx.tessellate(output.shapes, 1.0);
        }
         
         
        let world = [0.24, 0.28, 0.32, 1.0];
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
        let path = dir.join("clickgui-ambience.png");
        flat.save(&path).expect("the ambience preview saves");
        println!("clickgui ambience: {path:?}");
    }

     
     
     
     
    #[test]
    #[ignore = "writes a preview into screenshots/"]
    fn clickgui_gradient_preview() {
        use crate::module::registry::ModuleRegistry;

        let registry = ModuleRegistry::new();
        registry.register(crate::module::render::player_esp::PlayerEspModule::new());
        registry.register(crate::module::render::tracers::TracersModule::new());
        registry.register(crate::module::render::hand_aura::HandAuraModule::new());
        registry.register(crate::module::render::legit::coords());
        registry.register(crate::module::combat::killaura::KillAuraModule::new());
        registry.register(crate::module::movement::sprint::SprintModule::new());
         
        for handle in registry.handles() {
            handle
                .lock()
                .unwrap()
                .get_module_data_mut()
                .set_enabled(true);
        }
        crate::graphic::module_gradient::set_on(true);

        let map = registry.by_id();
        let render = render_panel_members(&map);
        let combat: Vec<Member> = map
            .iter()
            .filter(|(_, arc)| {
                arc.lock().unwrap().get_module_data().category == ModuleCategory::Combat
            })
            .map(|(id, arc)| Member {
                name: id.display_name(),
                arc,
            })
            .collect();

        let (width, height) = (640usize, 400usize);
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(width as f32, height as f32));
        let ctx = Context::default();
        let mut textures = HashMap::new();
        let mut primitives = Vec::new();
        for frame in 0..6 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                     
                     
                    time: Some(6.5 + frame as f64 / 60.0),
                    predicted_dt: 1.0 / 60.0,
                    ..Default::default()
                },
                |ctx| {
                    draw_backdrop(ctx, 1.0);
                    draw_panel(
                        ctx,
                        Chrome::resting(1.0),
                        ModuleCategory::Combat,
                        &combat,
                        pos2(28.0, 48.0),
                        &map,
                        0,
                    );
                    draw_panel(
                        ctx,
                        Chrome::resting(1.0),
                        ModuleCategory::Render,
                        &render,
                        pos2(210.0, 48.0),
                        &map,
                        1,
                    );
                },
            );
            gather_textures(&mut textures, &output.textures_delta);
            primitives = ctx.tessellate(output.shapes, 1.0);
        }
        let world = [0.24, 0.28, 0.32, 1.0];
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
        let path = dir.join("clickgui-gradient.png");
        flat.save(&path).expect("the gradient preview saves");
        println!("clickgui gradient: {path:?}");
         
        crate::graphic::module_gradient::set_on(false);
    }

     
     
     
    #[test]
    #[ignore = "writes a preview into screenshots/"]
    fn clickgui_sidebar_preview() {
        use crate::module::registry::ModuleRegistry;

        let registry = ModuleRegistry::new();
        registry.register(crate::module::render::player_esp::PlayerEspModule::new());
        registry.register(crate::module::render::tracers::TracersModule::new());
        registry.register(crate::module::render::hand_aura::HandAuraModule::new());
        registry.register(crate::module::render::smoke::SmokeModule::new());
        registry.register(crate::module::render::legit::coords());
        registry.register(crate::module::combat::killaura::KillAuraModule::new());
        registry.register(crate::module::movement::sprint::SprintModule::new());
        for id in [ModuleId::PlayerEsp, ModuleId::HandAura, ModuleId::Coords] {
            if let Some(arc) = registry.get(id) {
                arc.lock().unwrap().get_module_data_mut().set_enabled(true);
            }
        }

        let map = registry.by_id();
        let mut entries: Vec<(ModuleId, ModuleCategory)> = map
            .iter()
            .map(|(id, arc)| (*id, arc.lock().unwrap().get_module_data().category))
            .collect();
        entries.sort_by_key(|(id, _)| id.display_name());
        crate::graphic::menu_sidebar::set_selected(2);

         
        let (width, height) = (1920usize, 1080usize);
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(width as f32, height as f32));
        let ctx = Context::default();
        let mut textures = HashMap::new();
        let mut primitives = Vec::new();
        for frame in 0..40 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(4.0 + frame as f64 / 60.0),
                    predicted_dt: 1.0 / 60.0,
                    events: vec![egui::Event::PointerMoved(pos2(300.0, 120.0))],
                    ..Default::default()
                },
                |ctx| {
                    draw_backdrop(ctx, 1.0);
                    crate::graphic::menu_sidebar::draw(ctx, Chrome::resting(1.0), &entries, &map);
                },
            );
            gather_textures(&mut textures, &output.textures_delta);
            primitives = ctx.tessellate(output.shapes, 1.0);
        }
        let world = [0.24, 0.28, 0.32, 1.0];
        let image = rasterize(primitives.clone(), &textures, width, height, world);
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
        let path = dir.join("menu-sidebar.png");
        flat.save(&path).expect("the sidebar preview saves");
        println!("clickgui sidebar: {path:?}");

         
         
        ctx.data_mut(|d| {
            d.insert_temp(Id::new("menu_sidebar_query"), "a".to_owned());
        });
        for frame in 0..20 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(5.0 + frame as f64 / 60.0),
                    predicted_dt: 1.0 / 60.0,
                    ..Default::default()
                },
                |ctx| {
                    draw_backdrop(ctx, 1.0);
                    crate::graphic::menu_sidebar::draw(ctx, Chrome::resting(1.0), &entries, &map);
                },
            );
            gather_textures(&mut textures, &output.textures_delta);
            primitives = ctx.tessellate(output.shapes, 1.0);
        }
        let search = rasterize(primitives, &textures, width, height, world);
        let search = image::RgbaImage::from_fn(width as u32, height as u32, |x, y| {
            let pixel = search.get_pixel(x, y);
            let a = pixel[3] as f32 / 255.0;
            let over = |channel: usize| {
                ((pixel[channel] as f32 / 255.0 * a + world[channel] * (1.0 - a)) * 255.0).round()
                    as u8
            };
            image::Rgba([over(0), over(1), over(2), 255])
        });
        let path = dir.join("menu-sidebar-search.png");
        search.save(&path).expect("the search preview saves");
        println!("clickgui sidebar search: {path:?}");
    }

     
     
    #[test]
    fn the_retreat_follows_the_modal_open_factor() {
        assert_eq!(modal_retreat(0.0, 0.0, 0.0), 0.0);
        assert_eq!(modal_retreat(1.0, 0.0, 0.0), 1.0);
        assert_eq!(modal_retreat(0.0, 1.0, 0.0), 1.0);
        assert_eq!(modal_retreat(0.0, 0.0, 1.0), 1.0);
        assert!(modal_retreat(0.4, 0.0, 0.0) > modal_retreat(0.2, 0.0, 0.0));
         
        assert_eq!(modal_retreat(0.0, 0.7, 0.0), modal_retreat(0.7, 0.0, 0.0));
        assert_eq!(modal_retreat(0.0, 0.0, 0.7), modal_retreat(0.7, 0.0, 0.0));
        assert_eq!(modal_retreat(1.7, 0.0, 0.0), 1.0);
        assert_eq!(modal_retreat(-1.0, 0.0, 0.0), 0.0);
        assert_eq!(modal_retreat(f32::NAN, 0.0, 0.0), 1.0);
    }

     
     
    #[test]
    fn the_panels_retreat_while_a_modal_opens() {
        use crate::module::registry::ModuleRegistry;

        let registry = ModuleRegistry::new();
        registry.register(crate::module::combat::killaura::KillAuraModule::new());
        let map = registry.by_id();
        let members: Vec<Member> = map
            .iter()
            .map(|(id, arc)| Member {
                name: id.display_name(),
                arc,
            })
            .collect();

        let ctx = Context::default();
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 900.0));
        let mut painted = Vec::new();
        for frame in 0..4 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(frame as f64 / 60.0),
                    predicted_dt: 1.0 / 60.0,
                    ..Default::default()
                },
                |ctx| {
                    draw_panel(
                        ctx,
                        Chrome {
                            progress: 0.45,
                            retreat: 0.55,
                        },
                        ModuleCategory::Combat,
                        &members,
                        pos2(20.0, 20.0),
                        &map,
                        0,
                    )
                },
            );
            for clipped in output.shapes {
                labels_in(&clipped.shape, &mut painted);
            }
        }
        assert!(
            painted.iter().any(|label| label == "Combat"),
            "a panel mid-retreat is still drawn: {painted:?}"
        );

        let open = painted_labels(1.0, 0.0, 0.0);
        assert!(
            !open.iter().any(|label| label == "Combat"),
            "a fully open window leaves no panels behind it: {open:?}"
        );
        assert!(
            open.iter().any(|label| label == "Panic"),
            "the toolbar stays in front the whole time: {open:?}"
        );
    }

     
    fn labels_in(shape: &egui::Shape, out: &mut Vec<String>) {
        match shape {
            egui::Shape::Text(text) => out.push(text.galley.job.text.clone()),
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    labels_in(shape, out);
                }
            }
            _ => {}
        }
    }

     
     
    fn painted_labels(
        settings_progress: f32,
        friends_progress: f32,
        configs_progress: f32,
    ) -> Vec<String> {
        use egui::RawInput;

        let ctx = Context::default();
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1600.0, 1000.0));
        let mut painted = Vec::new();
        for frame in 0..20 {
            let output = ctx.run(
                RawInput {
                    screen_rect: Some(screen),
                    time: Some(frame as f64 / 60.0),
                    predicted_dt: 1.0 / 60.0,
                    ..Default::default()
                },
                |ctx| {
                    draw(
                        ctx,
                        1.0,
                        settings_progress,
                        friends_progress,
                        configs_progress,
                    )
                },
            );
            for clipped in output.shapes {
                labels_in(&clipped.shape, &mut painted);
            }
        }
        painted
    }

     
     
     
    #[test]
    fn settings_modal_keeps_the_toolbar_and_hides_the_panels() {
        let painted = painted_labels(1.0, 0.0, 0.0);

        assert!(
            painted.iter().any(|label| label == "Panic"),
            "the toolbar must stay reachable while Settings is open"
        );
        assert!(
            painted.iter().any(|label| label == "Settings"),
            "the Settings button must stay reachable"
        );
        assert!(
            !painted.iter().any(|label| label == "Combat"),
            "category panels must be hidden: {painted:?}"
        );
        assert!(
            !painted.iter().any(|label| label == "Search"),
            "the search panel must be hidden: {painted:?}"
        );
    }

     
    #[test]
    fn friends_modal_keeps_the_toolbar_and_hides_the_panels() {
        let painted = painted_labels(0.0, 1.0, 0.0);

        assert!(
            painted.iter().any(|label| label == "Panic"),
            "the toolbar must stay reachable while Friends is open"
        );
        assert!(
            !painted.iter().any(|label| label == "Combat"),
            "category panels must be hidden: {painted:?}"
        );
        assert!(
            !painted.iter().any(|label| label == "Search"),
            "the search panel must be hidden: {painted:?}"
        );
    }

     
     
    #[test]
    fn configs_modal_keeps_the_toolbar_and_hides_the_panels() {
        let painted = painted_labels(0.0, 0.0, 1.0);

        assert!(
            painted.iter().any(|label| label == "Panic"),
            "the toolbar must stay reachable while Configs is open: {painted:?}"
        );
        assert!(
            painted.iter().any(|label| label == "Configs"),
            "the Configs button must stay reachable: {painted:?}"
        );
        assert!(
            !painted.iter().any(|label| label == "Combat"),
            "category panels must be hidden: {painted:?}"
        );
        assert!(
            !painted.iter().any(|label| label == "Search"),
            "the search panel must be hidden: {painted:?}"
        );
    }
}
