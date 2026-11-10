 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::ambience;
use crate::graphic::anim::{self, Easing, SpringCfg};
use crate::graphic::branding;
use crate::graphic::menu::{self, ModuleMap};
use crate::graphic::theme;
use crate::graphic::transparency::{self, Layer};
use crate::module::{ModuleCategory, ModuleId};
use egui::{
    pos2, vec2, Align, Align2, Color32, Context, FontId, Id, Layout, Pos2, Rect, Response,
    Rounding, Sense, Stroke, Ui, UiBuilder, Vec2,
};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};

 
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MenuStyle {
     
    #[default]
    Panels,
     
    Sidebar,
}

 
 
static STYLE: AtomicU8 = AtomicU8::new(STYLE_PANELS);
const STYLE_PANELS: u8 = 0;
const STYLE_SIDEBAR: u8 = 1;
 
static SELECTED: AtomicU8 = AtomicU8::new(0);

 
 
 
const CATEGORIES: [ModuleCategory; 10] = [
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

 
 
 
const WIN_W: f32 = 700.0;
const WIN_H: f32 = 532.0;
 
 
 
const WIN_W_FRACTION: f32 = 0.75;
const WIN_H_FRACTION: f32 = 0.75;
 
const MIN_WIN_W: f32 = 560.0;
const MIN_WIN_H: f32 = 420.0;
const MIN_SCALE: f32 = 0.72;
const MAX_SCALE: f32 = 2.4;
 
const PAD: f32 = 18.0;
const HEADER_H: f32 = 62.0;
const RAIL_W: f32 = 202.0;
const RAIL_GAP: f32 = 12.0;
 
const ITEM_H: f32 = 40.0;
const ITEM_GAP: f32 = 4.0;
 
const SEARCH_H: f32 = 38.0;
 
const RAIL_FOOTER_H: f32 = 40.0;
 
const RAIL_HEAD_H: f32 = 30.0;
 
const RAIL_BAR_W: f32 = 3.0;
const RAIL_BAR_SHARE: f32 = 0.5;
 
const PANE_HEAD_H: f32 = 28.0;
 
const WINDOW_RADIUS: f32 = 20.0;
const SLAB_RADIUS: f32 = 15.0;
 
 

 
 
 
 
 
 
 
#[derive(Clone, Copy, Debug)]
struct Metrics {
    width: f32,
    height: f32,
    scale: f32,
}

impl Metrics {
     
     
    fn for_screen(screen: Rect) -> Self {
        let width = (screen.width() * WIN_W_FRACTION)
            .min(screen.width() - 24.0)
            .max(MIN_WIN_W.min(screen.width()));
        let height = (screen.height() * WIN_H_FRACTION)
            .min(screen.height() - 48.0)
            .max(MIN_WIN_H.min(screen.height()));
         
         
        let scale = (width / WIN_W)
            .min(height / WIN_H)
            .clamp(MIN_SCALE, MAX_SCALE);
        Self {
            width,
            height,
            scale,
        }
    }

     
    fn px(&self, design: f32) -> f32 {
        design * self.scale
    }
}

 
pub fn style() -> MenuStyle {
    match STYLE.load(Ordering::Relaxed) {
        STYLE_SIDEBAR => MenuStyle::Sidebar,
        _ => MenuStyle::Panels,
    }
}

 
pub fn set_style(style: MenuStyle) {
    let raw = match style {
        MenuStyle::Panels => STYLE_PANELS,
        MenuStyle::Sidebar => STYLE_SIDEBAR,
    };
    STYLE.store(raw, Ordering::Relaxed);
}

 
pub fn is_sidebar() -> bool {
    STYLE.load(Ordering::Relaxed) == STYLE_SIDEBAR
}

 
pub fn selected() -> usize {
    (SELECTED.load(Ordering::Relaxed) as usize).min(CATEGORIES.len() - 1)
}

 
pub fn set_selected(index: usize) {
    if index < CATEGORIES.len() {
        SELECTED.store(index as u8, Ordering::Relaxed);
    }
}

 
pub fn selected_category() -> ModuleCategory {
    CATEGORIES[selected()]
}

 
fn category_count(entries: &[(ModuleId, ModuleCategory)], category: ModuleCategory) -> usize {
    entries
        .iter()
        .filter(|(_, entry_category)| *entry_category == category)
        .count()
}

 
 
 
 
 
 
pub(crate) fn draw(
    ctx: &Context,
    chrome: crate::graphic::menu::Chrome,
    entries: &[(ModuleId, ModuleCategory)],
    registry: &ModuleMap,
) {
    let progress = chrome.progress.clamp(0.0, 1.0);
    if progress <= 0.001 {
        return;
    }
    let retreat = chrome.retreat.clamp(0.0, 1.0);
     
    ctx.request_repaint();

    let m = Metrics::for_screen(ctx.screen_rect());
    let (width, height) = (m.width, m.height);

     
     
    let query_id = Id::new("menu_sidebar_query");
    let mut query = ctx
        .data(|d| d.get_temp::<String>(query_id))
        .unwrap_or_default();

    let rise = 18.0 * (1.0 - Easing::Out.apply(progress));
     
     
    let drop = m.px(26.0) * retreat;
    let current = selected();
    egui::Area::new(Id::new("clickgui_sidebar"))
        .order(egui::Order::Foreground)
        .anchor(Align2::CENTER_CENTER, vec2(0.0, rise + drop))
        .show(ctx, |ui| {
            ui.set_opacity(progress);
            if retreat > 0.001 {
                ui.disable();
            }
            let (win, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());

            paint_window(ui, win, progress, m);

            let header = Rect::from_min_size(win.min, vec2(win.width(), m.px(HEADER_H)));
            paint_header(ui, header, entries.len(), progress, m);

            let rail = Rect::from_min_max(
                pos2(win.left() + m.px(PAD), win.top() + m.px(HEADER_H)),
                pos2(win.left() + m.px(PAD + RAIL_W), win.bottom() - m.px(PAD)),
            );
            paint_rail_back(ui, rail, m);

            let pane = Rect::from_min_max(
                pos2(rail.right() + m.px(RAIL_GAP), rail.top()),
                pos2(win.right() - m.px(PAD), win.bottom() - m.px(PAD)),
            );
            paint_pane_back(ui, pane, m);

            let menu_key = registry
                .get(&ModuleId::MenuHud)
                .map(|arc| arc.lock().unwrap().get_module_data().key_bind.to_string())
                .filter(|key| key != "None");

             
             
            let emphasis = if query.trim().is_empty() { 1.0 } else { 0.40 };
            if let Some(index) =
                draw_rail(ui, rail, entries, current, menu_key.as_deref(), emphasis, m)
            {
                 
                 
                set_selected(index);
                query.clear();
            }
             
             
            draw_pane(
                ui,
                pane.shrink2(vec2(m.px(12.0), m.px(10.0))),
                entries,
                registry,
                selected(),
                &mut query,
                m,
            );
        });

     
    ctx.data_mut(|d| d.insert_temp(query_id, query));
}

fn paint_window(ui: &Ui, win: Rect, progress: f32, m: Metrics) {
    let painter = ui.painter();
    let rounding = Rounding::same(m.px(WINDOW_RADIUS));

     
     
    for (grow, drop, alpha) in [
        (16.0_f32, 10.0, 10_u8),
        (10.0, 8.0, 16),
        (6.0, 6.0, 22),
        (3.0, 4.0, 28),
    ] {
        let grow = m.px(grow);
        painter.rect_filled(
            win.translate(vec2(0.0, m.px(drop))).expand(grow),
            Rounding::same(rounding.nw + grow),
            Color32::from_black_alpha(alpha),
        );
    }

    painter.rect_filled(win, rounding, transparency::surface(0.96, Layer::Window));
    painter.rect_filled(
        Rect::from_min_size(win.min, vec2(win.width(), m.px(1.0).max(1.0))),
        Rounding::ZERO,
        theme::with_alpha(theme::TEXT, 0.04),
    );
     
    transparency::edge(painter, win, rounding, Layer::Window, progress);
}

 
 
 
fn paint_header(ui: &Ui, header: Rect, total: usize, progress: f32, m: Metrics) {
    let painter = ui.painter();
    let now = ui.input(|i| i.time);
    let logo_center = pos2(
        header.left() + m.px(PAD + 16.0),
        header.center().y - m.px(2.0),
    );

    let word_left = match branding::menu_logo_texture(ui.ctx()) {
        Some(logo) => {
             
            ambience::halo(
                painter,
                logo_center,
                m.px(32.0),
                theme::brand(),
                (0.12 + 0.035 * (now as f32 * 1.3).sin()) * progress,
            );
            let size = logo.size_vec2();
            let fit = Vec2::splat(m.px(28.0));
            let fit_scale = (fit.x / size.x).min(fit.y / size.y);
            painter.image(
                logo.id(),
                Rect::from_center_size(logo_center, size * fit_scale),
                Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
                theme::with_alpha(theme::brand(), progress),
            );
            header.left() + m.px(PAD + 42.0)
        }
        None => header.left() + m.px(PAD),
    };

    painter.text(
        pos2(word_left, header.center().y - m.px(8.0)),
        Align2::LEFT_CENTER,
        "Infered",
        FontId::proportional(m.px(19.0)),
        theme::TEXT,
    );
    painter.text(
        pos2(word_left, header.center().y + m.px(11.0)),
        Align2::LEFT_CENTER,
        "CLICKGUI",
        FontId::proportional(m.px(9.5)),
        theme::with_alpha(theme::accent(), 0.95),
    );

     
     
    painter.text(
        pos2(header.right() - m.px(PAD), header.center().y),
        Align2::RIGHT_CENTER,
        format!("{total} modules"),
        FontId::proportional(m.px(11.0)),
        theme::TEXT_MUTED,
    );
}

 
 
fn paint_rail_back(ui: &Ui, rail: Rect, m: Metrics) {
     
     
    let fill = transparency::fill(
        theme::lerp_color(theme::SURFACE, theme::SURFACE_HOVER, 0.55),
        0.95,
        Layer::Panel,
    );
    ui.painter()
        .rect_filled(rail, Rounding::same(m.px(SLAB_RADIUS)), fill);
}

 
 
fn paint_pane_back(ui: &Ui, pane: Rect, m: Metrics) {
     
     
    let fill = transparency::fill(theme::SURFACE, 0.80, Layer::Pane);
    ui.painter()
        .rect_filled(pane, Rounding::same(m.px(SLAB_RADIUS)), fill);
}

 
 
fn draw_rail(
    ui: &mut Ui,
    rail: Rect,
    entries: &[(ModuleId, ModuleCategory)],
    selected: usize,
    menu_key: Option<&str>,
    emphasis: f32,
    m: Metrics,
) -> Option<usize> {
    let gap = m.px(ITEM_GAP);
    let head_h = m.px(RAIL_HEAD_H);
     
     
    let row_h = ((rail.height() - head_h - m.px(RAIL_FOOTER_H)) / CATEGORIES.len() as f32 - gap)
        .clamp(m.px(26.0), m.px(ITEM_H));
    paint_rail_heading(ui, rail, m);
    let mut clicked = None;
    ui.allocate_new_ui(
        UiBuilder::new()
            .max_rect(rail)
            .layout(Layout::top_down(Align::Min)),
        |ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
             
             
            let rows = CATEGORIES.len() as f32 * (row_h + gap) - gap;
            let slack = (rail.height() - head_h - rows - m.px(RAIL_FOOTER_H)).max(0.0);
            let top = rail.top() + head_h + slack * 0.32;
            ui.add_space(top - rail.top());
             
             
            let target = top + selected as f32 * (row_h + gap);
            let pill_y = anim::spring(
                ui.ctx(),
                Id::new("menu_sidebar_rail_pill"),
                target,
                SpringCfg::PANEL,
            );
            let pill = Rect::from_min_size(pos2(rail.left(), pill_y), vec2(rail.width(), row_h));
            paint_rail_pill(ui.painter(), pill, emphasis);

            for (index, category) in CATEGORIES.iter().enumerate() {
                let count = category_count(entries, *category);
                if index > 0 {
                    ui.add_space(gap);
                }
                if rail_item(ui, *category, index == selected, count, row_h, emphasis, m).clicked()
                {
                    clicked = Some(index);
                }
            }

            paint_rail_footer(ui, rail, menu_key, m);
        },
    );
    clicked
}

 
fn paint_rail_heading(ui: &Ui, rail: Rect, m: Metrics) {
    ui.painter().text(
        pos2(
            rail.left() + m.px(16.0),
            rail.top() + m.px(RAIL_HEAD_H) * 0.5,
        ),
        Align2::LEFT_CENTER,
        "CATEGORIES",
        FontId::proportional(m.px(9.5)),
        theme::with_alpha(theme::TEXT_MUTED, 0.9),
    );
}

 
fn paint_rail_footer(ui: &Ui, rail: Rect, key: Option<&str>, m: Metrics) {
    let painter = ui.painter();
    let key = key.unwrap_or("R-SHIFT");
    let key_width = ui.fonts(|fonts| {
        fonts
            .layout_no_wrap(
                key.to_owned(),
                FontId::proportional(m.px(9.5)),
                Color32::WHITE,
            )
            .size()
            .x
    });
    let cap = Rect::from_min_size(
        pos2(rail.left() + m.px(16.0), rail.bottom() - m.px(34.0)),
        vec2(key_width + m.px(18.0), m.px(23.0)),
    );
    painter.rect_filled(
        cap,
        Rounding::same(cap.height() * 0.5),
        theme::lerp_color(theme::SURFACE_HOVER, theme::accent(), 0.12),
    );
    painter.text(
        cap.center(),
        Align2::CENTER_CENTER,
        key,
        FontId::proportional(m.px(10.0)),
        theme::TEXT_DIM,
    );
    painter.text(
        pos2(cap.right() + m.px(10.0), cap.center().y),
        Align2::LEFT_CENTER,
        "closes the menu",
        FontId::proportional(m.px(10.0)),
        theme::TEXT_MUTED,
    );
}

 
 
fn paint_rail_pill(painter: &egui::Painter, pill: Rect, emphasis: f32) {
    let emphasis = emphasis.clamp(0.0, 1.0);
    let round = Rounding::same(pill.height() * 0.5);
    ambience::halo(
        painter,
        pill.left_center(),
        46.0,
        theme::accent(),
        0.10 * emphasis,
    );
    painter.rect_filled(
        pill,
        round,
        theme::with_alpha(theme::accent(), 0.14 * emphasis),
    );
     
    let bar_h = pill.height() * RAIL_BAR_SHARE;
    let bar = Rect::from_min_size(
        pos2(pill.left(), pill.center().y - bar_h * 0.5),
        vec2(RAIL_BAR_W, bar_h),
    );
    painter.rect_filled(
        bar,
        Rounding::same(RAIL_BAR_W * 0.5),
        theme::with_alpha(theme::accent(), emphasis),
    );
}

 
fn rail_item(
    ui: &mut Ui,
    category: ModuleCategory,
    selected: bool,
    count: usize,
    row_h: f32,
    emphasis: f32,
    m: Metrics,
) -> Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(vec2(width, row_h), Sense::click());
    let ctx = ui.ctx().clone();
    let id = Id::new("menu_sidebar_rail").with(category.display_name());

    let hover = anim::toggle(
        &ctx,
        id.with("hover"),
        response.hovered(),
        0.14,
        Easing::Out,
    );
    let active = anim::toggle(&ctx, id.with("active"), selected, 0.24, Easing::Out)
        * emphasis.clamp(0.0, 1.0);
    let press = anim::toggle(
        &ctx,
        id.with("press"),
        response.is_pointer_button_down_on(),
        0.08,
        Easing::Out,
    );
    let tint = theme::category_accent(category);
    let lift = hover.max(active);

    let painter = ui.painter();
    let pill = Rounding::same(rect.height() * 0.5);
    if hover > 0.001 {
        painter.rect_filled(
            rect,
            pill,
            theme::with_alpha(theme::SURFACE_HOVER, 0.55 * hover * emphasis),
        );
    }

     
     
    let scale = 1.4 * m.scale * (1.0 + 0.10 * hover + 0.05 * active - 0.07 * press);
    let icon_color = theme::lerp_color(
        theme::TEXT_MUTED,
        if selected {
            theme::accent()
        } else {
            theme::TEXT
        },
        lift.clamp(0.0, 1.0),
    );
    menu::paint_category_glyph_scaled(
        painter,
        pos2(rect.left() + m.px(24.0), rect.center().y),
        category,
        icon_color,
        scale,
    );

     
     
    let presence = if count == 0 { 0.55 } else { 1.0 };
    painter.text(
        pos2(rect.left() + m.px(46.0), rect.center().y),
        Align2::LEFT_CENTER,
        category.display_name(),
        FontId::proportional(m.px(14.0)),
        theme::with_alpha(
            theme::lerp_color(theme::TEXT_DIM, theme::TEXT, lift.clamp(0.0, 1.0)),
            presence,
        ),
    );

     
    if count > 0 {
        painter.text(
            pos2(rect.right() - m.px(14.0), rect.center().y),
            Align2::RIGHT_CENTER,
            count.to_string(),
            FontId::proportional(m.px(10.5)),
            theme::with_alpha(
                theme::lerp_color(theme::TEXT_MUTED, tint, 0.25 + 0.5 * lift),
                presence,
            ),
        );
    }

    response
}

 
fn draw_pane(
    ui: &mut Ui,
    pane: Rect,
    entries: &[(ModuleId, ModuleCategory)],
    registry: &ModuleMap,
    selected: usize,
    query: &mut String,
    m: Metrics,
) {
    let needle = query.trim().to_lowercase();
    let searching = !needle.is_empty();
    let items: Vec<(ModuleId, ModuleCategory)> = if searching {
         
         
        entries
            .iter()
            .filter(|(id, _)| menu::search_matches(&needle, id.display_name()) || (needle == "*" && crate::config::is_favorite(*id)))
            .copied()
            .collect()
    } else {
        entries
            .iter()
            .filter(|(_, category)| *category == CATEGORIES[selected])
            .copied()
            .collect()
    };

    let width = pane.width();
    ui.allocate_new_ui(
        UiBuilder::new()
            .max_rect(pane)
            .layout(Layout::top_down(Align::Min)),
        |ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;

            let (head, _) = ui.allocate_exact_size(vec2(width, m.px(PANE_HEAD_H)), Sense::hover());
            paint_pane_header(ui, head, selected, searching, items.len(), m);

            ui.add_space(m.px(10.0));
            search_shell(ui, query, width, needle.len(), m);
            if ui.small_button("★ Favorites").clicked() {
                *query = "*".into();
            }
            ui.add_space(m.px(10.0));

            let list_h = (ui.available_height() - m.px(4.0)).max(m.px(80.0));
            draw_results(
                ui,
                list_h,
                &items,
                registry,
                selected,
                searching.then_some(needle.as_str()),
                m,
            );
        },
    );
}

 
 
fn paint_pane_header(
    ui: &Ui,
    rect: Rect,
    selected: usize,
    searching: bool,
    count: usize,
    m: Metrics,
) {
    let painter = ui.painter();
    let glyph = if searching {
        ModuleCategory::Search
    } else {
        CATEGORIES[selected]
    };
    let title = if searching {
        "Search".to_owned()
    } else {
        CATEGORIES[selected].display_name().to_owned()
    };

    menu::paint_category_glyph_scaled(
        painter,
        pos2(rect.left() + m.px(8.0), rect.center().y),
        glyph,
        theme::accent(),
        1.22 * m.scale,
    );
    painter.text(
        pos2(rect.left() + m.px(23.0), rect.center().y),
        Align2::LEFT_CENTER,
        &title,
        FontId::proportional(m.px(16.0)),
        theme::TEXT,
    );

    let detail = if searching {
        match count {
            1 => "1 match, every category".to_owned(),
            count => format!("{count} matches, every category"),
        }
    } else {
        match count {
            1 => "1 module".to_owned(),
            count => format!("{count} modules"),
        }
    };
    painter.text(
        pos2(rect.right(), rect.center().y),
        Align2::RIGHT_CENTER,
        detail,
        FontId::proportional(m.px(10.5)),
        theme::TEXT_MUTED,
    );

     
     
}

 
 
fn search_shell(ui: &mut Ui, query: &mut String, width: f32, chars: usize, m: Metrics) {
    let id = Id::new("menu_sidebar_search");
    let ctx = ui.ctx().clone();
    let shell = Rect::from_min_size(ui.cursor().min, vec2(width, m.px(SEARCH_H)));

    let hover = anim::toggle(
        &ctx,
        id.with("hover"),
        ui.rect_contains_pointer(shell),
        0.14,
        Easing::Out,
    );
    let focused = ctx.memory(|memory| memory.has_focus(id.with("field")));
    let focus = anim::toggle(&ctx, id.with("focus"), focused, 0.16, Easing::Out);

    {
        let painter = ui.painter();
         
         
        painter.rect_filled(
            shell,
            Rounding::same(shell.height() * 0.5),
            theme::lerp_color(
                theme::lerp_color(theme::SURFACE_HOVER, theme::TEXT_MUTED, 0.12),
                theme::with_alpha(theme::accent(), 0.16),
                focus.max(hover * 0.35),
            ),
        );
        if focus > 0.001 {
            ambience::halo(
                painter,
                shell.left_center(),
                shell.height() * 1.6,
                theme::accent(),
                0.10 * focus,
            );
        }
         
        let glyph_scale = 1.16 * m.scale * (1.0 + 0.06 * hover);
        menu::paint_category_glyph_scaled(
            painter,
            pos2(shell.left() + m.px(20.0), shell.center().y),
            ModuleCategory::Search,
            theme::lerp_color(theme::TEXT_MUTED, theme::accent(), focus.max(hover * 0.5)),
            glyph_scale,
        );
    }

    let text_rect = Rect::from_min_max(
        pos2(shell.left() + m.px(38.0), shell.top() + m.px(7.0)),
        pos2(shell.right() - m.px(38.0), shell.bottom() - m.px(7.0)),
    );
    ui.put(
        text_rect,
        egui::TextEdit::singleline(query)
            .id(id.with("field"))
            .frame(false)
            .font(FontId::proportional(m.px(14.0)))
            .hint_text("Search every category…")
            .desired_width(text_rect.width()),
    );

     
     
    if !query.is_empty() {
        let clear = Rect::from_center_size(
            pos2(shell.right() - m.px(20.0), shell.center().y),
            Vec2::splat(m.px(24.0)),
        );
        let clear_response = ui.interact(clear, id.with("clear"), Sense::click());
        let clear_hover = anim::toggle(
            &ctx,
            id.with("clear_hover"),
            clear_response.hovered(),
            0.12,
            Easing::Out,
        );
        let painter = ui.painter();
        painter.circle_filled(
            clear.center(),
            m.px(9.0),
            theme::with_alpha(theme::SURFACE_HOVER, 0.45 + 0.55 * clear_hover),
        );
        let cross = m.px(3.0);
        let color = theme::lerp_color(theme::TEXT_MUTED, theme::TEXT, clear_hover);
        let stroke = Stroke::new(m.px(1.2).max(1.0), color);
        painter.line_segment(
            [
                clear.center() + vec2(-cross, -cross),
                clear.center() + vec2(cross, cross),
            ],
            stroke,
        );
        painter.line_segment(
            [
                clear.center() + vec2(-cross, cross),
                clear.center() + vec2(cross, -cross),
            ],
            stroke,
        );
        if clear_response.clicked() {
            query.clear();
        }
    }

     
     
    ui.advance_cursor_after_rect(shell);

     
    if focused || chars > 0 {
        ctx.request_repaint();
    }
}

 
 
fn draw_results(
    ui: &mut Ui,
    height: f32,
    items: &[(ModuleId, ModuleCategory)],
    registry: &ModuleMap,
    selected: usize,
    needle: Option<&str>,
    m: Metrics,
) {
    let searching = needle.is_some();
    let now = ui.input(|i| i.time);
    let key = match needle {
        Some(query) => format!("q:{query}"),
        None => format!("c:{selected}"),
    };
    let motion_id = Id::new("menu_sidebar_list_motion");
    let (stored, start) = ui
        .ctx()
        .data(|d| d.get_temp::<(String, f64)>(motion_id))
        .unwrap_or_default();
    let start = if stored == key {
        start
    } else {
        ui.ctx()
            .data_mut(|d| d.insert_temp(motion_id, (key.clone(), now)));
        now
    };
    let age = (now - start).max(0.0) as f32;

     
    let mut scroll = egui::style::ScrollStyle::floating();
    scroll.bar_width = m.px(5.0);
    ui.style_mut().spacing.scroll = scroll;
     
     
    let active_fg = ui.visuals().widgets.active.fg_stroke.color;
    ui.visuals_mut().widgets.active.fg_stroke.color = theme::accent();

    egui::ScrollArea::vertical()
        .id_salt("menu_sidebar_results")
        .max_height(height)
        .auto_shrink([false, true])
        .drag_to_scroll(false)
        .show(ui, |ui| {
            ui.visuals_mut().widgets.active.fg_stroke.color = active_fg;

            if items.is_empty() {
                empty_state(ui, searching, selected, m);
                return;
            }

            let row_width = ui.available_width().max(m.px(160.0));
            for (index, (id, category)) in items.iter().enumerate() {
                let Some(arc) = registry.get(id) else {
                    continue;
                };
                let alpha = row_alpha(age, index);
                if alpha < 0.999 {
                     
                     
                     
                    ui.add_space((1.0 - alpha) * m.px(9.0));
                }
                ui.scope(|ui| {
                    ui.set_opacity(alpha);
                    menu::draw_module_row(
                        ui,
                        id.display_name(),
                        arc,
                        registry,
                        row_width,
                        searching.then_some(*category),
                        m.scale,
                    );
                });
            }
            ui.add_space(m.px(9.0));
        });
}

 
 
 
fn row_alpha(age: f32, index: usize) -> f32 {
    let delay = (index.min(14) as f32) * 0.022;
    let t = ((age - delay) / 0.22).clamp(0.0, 1.0);
    Easing::Out.apply(t)
}

 
fn empty_state(ui: &mut Ui, searching: bool, selected: usize, m: Metrics) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, m.px(190.0)), Sense::hover());
    let now = ui.input(|i| i.time) as f32;
    let painter = ui.painter();
    let centre = pos2(rect.center().x, rect.top() + m.px(58.0));
     
     
    let pulse = 1.0 + 0.03 * (now * 1.8).sin();
    painter.circle_filled(
        centre,
        m.px(26.0) * pulse,
        theme::with_alpha(theme::accent(), 0.05),
    );
    painter.circle_filled(centre, m.px(24.0), theme::with_alpha(theme::SURFACE, 0.60));
    let glyph = if searching {
        ModuleCategory::Search
    } else {
        CATEGORIES[selected]
    };
    menu::paint_category_glyph_scaled(
        painter,
        centre,
        glyph,
        theme::with_alpha(theme::accent(), 0.85),
        2.2 * m.scale,
    );

    let (title, detail) = if searching {
        (
            "No modules match",
            "Try a shorter word — the search covers every category.",
        )
    } else {
        (
            "Nothing here yet",
            "Modules in this category will appear here.",
        )
    };
    painter.text(
        pos2(centre.x, centre.y + m.px(46.0)),
        Align2::CENTER_CENTER,
        title,
        FontId::proportional(m.px(13.5)),
        theme::TEXT_DIM,
    );
    painter.text(
        pos2(centre.x, centre.y + m.px(64.0)),
        Align2::CENTER_CENTER,
        detail,
        FontId::proportional(m.px(11.0)),
        theme::TEXT_MUTED,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
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

     
     
    fn globals() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

     
    fn test_world() -> (Vec<(ModuleId, ModuleCategory)>, ModuleMap) {
        use crate::module::registry::ModuleRegistry;

        let registry = ModuleRegistry::new();
        registry.register(crate::module::combat::killaura::KillAuraModule::new());
        registry.register(crate::module::movement::sprint::SprintModule::new());
        registry.register(crate::module::render::player_esp::PlayerEspModule::new());

        let map = registry.by_id();
        let mut entries: Vec<(ModuleId, ModuleCategory)> = map
            .iter()
            .map(|(id, arc)| (*id, arc.lock().unwrap().get_module_data().category))
            .collect();
        entries.sort_by_key(|(id, _)| id.display_name());
        (entries, map)
    }

     
     
     
    fn draw_and_collect(
        ctx: &Context,
        entries: &[(ModuleId, ModuleCategory)],
        map: &ModuleMap,
        frames: usize,
    ) -> Vec<String> {
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 800.0));
        let mut drawn = Vec::new();
         
         
        for frame in 0..frames {
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
                        crate::graphic::menu::Chrome::resting(1.0),
                        entries,
                        map,
                    )
                },
            );
            if frame + 1 == frames {
                for clipped in &output.shapes {
                    collect(&clipped.shape, &mut drawn);
                }
            }
        }
        drawn
    }

     
     
    #[test]
    fn the_rail_lists_every_category_and_the_pane_follows_the_selection() {
        let _guard = globals();
        let (entries, map) = test_world();
        let ctx = Context::default();
        set_selected(0);
        let drawn = draw_and_collect(&ctx, &entries, &map, 30);

        for category in CATEGORIES {
            assert!(
                drawn.iter().any(|text| text == category.display_name()),
                "the rail shows {}: {drawn:?}",
                category.display_name()
            );
        }
        let combat = ModuleId::KillAura.display_name();
        let movement = ModuleId::Sprint.display_name();
        assert!(
            drawn.iter().any(|text| text == combat),
            "the selected category's modules are listed: {drawn:?}"
        );
        assert!(
            !drawn.iter().any(|text| text == movement),
            "modules from other categories stay out of the pane: {drawn:?}"
        );
    }

     
     
    #[test]
    fn searching_lists_matches_from_other_categories() {
        let _guard = globals();
        let (entries, map) = test_world();
        let ctx = Context::default();
        set_selected(0);
        ctx.data_mut(|d| d.insert_temp(Id::new("menu_sidebar_query"), "sprint".to_owned()));
        let drawn = draw_and_collect(&ctx, &entries, &map, 30);

        assert!(
            drawn.iter().any(|text| text == "Search"),
            "the pane header switches to the search: {drawn:?}"
        );
        assert!(
            drawn
                .iter()
                .any(|text| text == ModuleId::Sprint.display_name()),
            "the Movement module matches from the Combat selection: {drawn:?}"
        );
        assert!(
            !drawn
                .iter()
                .any(|text| text == ModuleId::KillAura.display_name()),
            "non-matching modules are filtered out: {drawn:?}"
        );
    }

    #[test]
    fn search_matches_reordered_and_unspaced_names() {
        let _guard = globals();
        let (entries, map) = test_world();
        for query in ["esp player", "playeresp"] {
            let ctx = Context::default();
            set_selected(0);
            ctx.data_mut(|d| d.insert_temp(Id::new("menu_sidebar_query"), query.to_owned()));
            let drawn = draw_and_collect(&ctx, &entries, &map, 30);
            assert!(
                drawn
                    .iter()
                    .any(|text| text == ModuleId::PlayerEsp.display_name()),
                "{query}: {drawn:?}"
            );
            assert!(
                !drawn
                    .iter()
                    .any(|text| text == ModuleId::KillAura.display_name()),
                "{query}: {drawn:?}"
            );
        }
    }

     
     
    #[test]
    fn rows_stagger_in_and_settle() {
        assert_eq!(row_alpha(0.0, 0), 0.0);
        assert_eq!(row_alpha(0.0, 5), 0.0);
        assert_eq!(row_alpha(10.0, 40), 1.0);
        assert!(row_alpha(0.06, 0) > row_alpha(0.06, 4));
        for index in 0..20 {
            let end = 0.22 + index.min(14) as f32 * 0.022 + 1e-3;
            assert!((row_alpha(end, index) - 1.0).abs() < 0.01);
        }
    }

     
     
     
    #[test]
    fn the_window_fills_three_quarters_of_the_screen() {
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1920.0, 1080.0));
        let m = Metrics::for_screen(screen);
        assert!((m.width - 1440.0).abs() < 0.5, "width was {}", m.width);
        assert!((m.height - 810.0).abs() < 0.5, "height was {}", m.height);
        assert!(
            m.scale > 1.4,
            "the interior grows with the window: {}",
            m.scale
        );
        assert!(m.width <= screen.width() && m.height <= screen.height());

        let small = Metrics::for_screen(Rect::from_min_size(Pos2::ZERO, Vec2::new(640.0, 480.0)));
        assert!(
            small.width <= 640.0 && small.height <= 480.0,
            "the window fits a small screen: {}×{}",
            small.width,
            small.height
        );
        assert!((MIN_SCALE..=1.0).contains(&small.scale), "{}", small.scale);
    }

     
     
    #[test]
    fn the_window_sinks_behind_a_modal() {
        let _guard = globals();
        let (entries, map) = test_world();
        let centre_y = |retreat: f32| {
            let ctx = Context::default();
            let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1920.0, 1080.0));
            let mut centre = None;
            for frame in 0..20 {
                let _ = ctx.run(
                    RawInput {
                        screen_rect: Some(screen),
                        time: Some(frame as f64 / 60.0),
                        predicted_dt: 1.0 / 60.0,
                        ..Default::default()
                    },
                    |ctx| {
                        draw(
                            ctx,
                            crate::graphic::menu::Chrome {
                                progress: 1.0,
                                retreat,
                            },
                            &entries,
                            &map,
                        )
                    },
                );
                if let Some(rect) =
                    ctx.memory(|memory| memory.area_rect(Id::new("clickgui_sidebar")))
                {
                    centre = Some(rect.center().y);
                }
            }
            centre.expect("the window was laid out")
        };

        let resting = centre_y(0.0);
        let behind = centre_y(0.6);
        assert!(
            behind > resting + 4.0,
            "the window sinks behind the modal: {resting} -> {behind}"
        );
    }

    #[test]
    fn style_and_selection_round_trip() {
        let _guard = globals();
        set_style(MenuStyle::Sidebar);
        assert!(is_sidebar());
        assert_eq!(style(), MenuStyle::Sidebar);
        set_selected(3);
        assert_eq!(selected(), 3);
        assert_eq!(selected_category(), CATEGORIES[3]);
        set_selected(99);
        assert_eq!(selected(), 3);
        set_style(MenuStyle::Panels);
        assert!(!is_sidebar());
        assert_eq!(style(), MenuStyle::Panels);
    }
}
