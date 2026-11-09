 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use egui::{
    pos2, vec2, Align, Color32, CursorIcon, Id, Painter, Pos2, Rect, Rounding, Sense, Shape,
    Stroke, Ui, Vec2,
};

use super::theme::{lerp_color, pulse, with_alpha, Palette};
use super::widgets::{
    ease_out_back, elided, radial_glow, top_light_rounded, tracked, tracked_width,
};
use super::{ease_out_cubic, DrawerPage, InjectorGui};

 
const GAP: f32 = 12.0;
 
const HEADER_H: f32 = 44.0;
 
const GRID_TOP: f32 = 56.0;

 
#[derive(Clone, Copy, PartialEq, Eq)]
enum Icon {
     
    Sliders,
     
    People,
     
    Play,
     
    Tiles,
}

 
struct Entry {
    page: DrawerPage,
    title: &'static str,
    detail: &'static str,
    icon: Icon,
}

 
 
const ENTRIES: [Entry; 4] = [
    Entry {
        page: DrawerPage::Config,
        title: "CONFIG",
        detail: "Modules, settings and profiles",
        icon: Icon::Sliders,
    },
    Entry {
        page: DrawerPage::Friends,
        title: "FRIENDS",
        detail: "Players to leave alone",
        icon: Icon::People,
    },
    Entry {
        page: DrawerPage::Macros,
        title: "MACROS",
        detail: "Automate with .macro files",
        icon: Icon::Play,
    },
    Entry {
        page: DrawerPage::Addons,
        title: "ADDONS",
        detail: "Load .inf extensions",
        icon: Icon::Tiles,
    },
];

 
pub(super) fn page(gui: &mut InjectorGui, ui: &mut Ui, now: f64) {
    let (area, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
    if area.width() < 120.0 || area.height() < 120.0 {
        return;
    }
    let entered = (now - gui.page_entered_at).max(0.0);

    backdrop(ui, area, now);
    header(ui, area, now, entered);

     
     
    let grid_top = area.top() + GRID_TOP;
    let grid_bottom = (area.bottom() - 1.0).max(grid_top + 60.0);
    let tile_w = ((area.width() - GAP) * 0.5).max(120.0);
    let tile_h = ((grid_bottom - grid_top - GAP) * 0.5).max(52.0);

    let mut open = None;
    for (index, entry) in ENTRIES.iter().enumerate() {
        let col = (index % 2) as f32;
        let row = (index / 2) as f32;
        let slot = Rect::from_min_size(
            pos2(
                area.left() + col * (tile_w + GAP),
                grid_top + row * (tile_h + GAP),
            ),
            vec2(tile_w, tile_h),
        );
        if tile(ui, slot, entry, index, now, entered) {
            open = Some(entry.page);
        }
    }

    if let Some(page) = open {
        gui.open_page(page, now, true);
    }
}

 

 
 
 
fn backdrop(ui: &Ui, area: Rect, now: f64) {
    let pointer = ui
        .ctx()
        .pointer_hover_pos()
        .filter(|pos| area.contains(*pos));
    let centre = area.center();
    let size = area.size();
    let target = pointer
        .map(|pos| {
            vec2(
                (pos.x - centre.x) / size.x.max(1.0),
                (pos.y - centre.y) / size.y.max(1.0),
            )
        })
        .unwrap_or(Vec2::ZERO);

    let ctx = ui.ctx();
    let drift_x = ctx.animate_value_with_time(Id::new("menu_glow_x"), target.x, 0.55);
    let drift_y = ctx.animate_value_with_time(Id::new("menu_glow_y"), target.y, 0.55);
    let drift = vec2(drift_x * 14.0, drift_y * 9.0);

    let painter = ui.painter();
    let breathe = 0.75 + 0.25 * pulse(now, 6.0);
    radial_glow(
        painter,
        area.left_top() + vec2(area.width() * 0.16, -34.0) + drift,
        area.width() * 0.42,
        Palette::ACCENT,
        0.055 * breathe,
    );
    radial_glow(
        painter,
        area.right_bottom() + vec2(-40.0, 34.0) - drift,
        area.width() * 0.30,
        Palette::ACCENT_DEEP,
        0.16 * breathe,
    );

     
    for i in 0..12 {
        let x = area.left() + hash01(i * 3 + 1) * area.width();
        let y = area.top() + hash01(i * 3 + 2) * area.height();
        let twinkle = 0.5 + 0.5 * (now as f32 * 0.6 + i as f32 * 1.9).sin();
        let radius = 0.6 + 0.5 * hash01(i * 3 + 3);
        painter.circle_filled(
            pos2(x, y),
            radius,
            with_alpha(Palette::TEXT, 0.035 + 0.055 * twinkle),
        );
    }
}

 

 
fn header(ui: &Ui, area: Rect, now: f64, entered: f64) {
    let draw = ease_out_cubic((entered / 0.44).clamp(0.0, 1.0) as f32);
    let mut painter = ui.painter().clone();
    painter.set_opacity(draw);

     
    let mark = Rect::from_min_size(pos2(area.left() + 1.0, area.top() + 2.0), Vec2::splat(32.0));
    sigil(&painter, mark, now);

    let text_x = mark.right() + 13.0;
    tracked(
        &painter,
        "MENU",
        17.0,
        Palette::TEXT,
        Align::Min,
        text_x,
        mark.center().y - 5.0,
    );
    tracked(
        &painter,
        "COMMAND CENTER",
        7.5,
        with_alpha(Palette::TEXT_DIM, 0.66),
        Align::Min,
        text_x,
        mark.center().y + 10.0,
    );

     
     
    let y = area.top() + HEADER_H;
    let full = (area.width() - 2.0).max(0.0);
    painter.rect_filled(
        Rect::from_min_size(pos2(area.left() + 1.0, y), vec2(full, 1.0)),
        Rounding::same(1.0),
        with_alpha(Palette::TEXT_DIM, 0.11),
    );
    painter.rect_filled(
        Rect::from_min_size(pos2(area.left() + 1.0, y - 0.25), vec2(full * draw, 1.5)),
        Rounding::same(1.0),
        with_alpha(Palette::ACCENT, 0.34 + 0.24 * pulse(now, 3.4)),
    );
    painter.circle_filled(
        pos2(area.left() + 1.0 + full * draw, y),
        2.0,
        with_alpha(Palette::ACCENT_BRIGHT, 0.9 * (1.0 - draw)),
    );
}

 
 
 
fn sigil(painter: &Painter, rect: Rect, now: f64) {
    let centre = rect.center();
    let breathe = pulse(now, 3.6);
    radial_glow(
        painter,
        centre,
        20.0,
        Palette::ACCENT,
        0.14 + 0.08 * breathe,
    );
    painter.circle_stroke(
        centre,
        13.2,
        Stroke::new(1.0_f32, with_alpha(Palette::ACCENT, 0.24 + 0.12 * breathe)),
    );

     
    let start = (now * 0.9) as f32;
    let end = start + 1.9;
    let arc: Vec<Pos2> = (0..=18)
        .map(|i| {
            let angle = start + (end - start) * i as f32 / 18.0;
            centre + vec2(angle.cos(), angle.sin()) * 13.2
        })
        .collect();
    painter.add(Shape::line(
        arc,
        Stroke::new(1.6_f32, with_alpha(Palette::ACCENT_BRIGHT, 0.95)),
    ));
    painter.circle_filled(
        centre + vec2(end.cos(), end.sin()) * 13.2,
        1.7,
        Palette::ACCENT_BRIGHT,
    );

    let radius = 6.2;
    painter.add(Shape::convex_polygon(
        vec![
            centre + vec2(0.0, -radius),
            centre + vec2(radius, 0.0),
            centre + vec2(0.0, radius),
            centre + vec2(-radius, 0.0),
        ],
        with_alpha(Palette::ACCENT, 0.92),
        Stroke::new(0.0_f32, Color32::TRANSPARENT),
    ));
    painter.circle_filled(
        centre,
        1.7 + 0.3 * breathe,
        with_alpha(Color32::WHITE, 0.88),
    );
}

 

 
const WELL: f32 = 44.0;
 
const ICON_GAP: f32 = 13.0;

 
 
 
fn tile_layout(rect: Rect, copy_w: f32) -> (Pos2, f32) {
    let left = rect.center().x - (WELL + ICON_GAP + copy_w) * 0.5;
    (
        pos2(left + WELL * 0.5, rect.center().y),
        left + WELL + ICON_GAP,
    )
}

 
fn tile(ui: &Ui, slot: Rect, entry: &Entry, index: usize, now: f64, entered: f64) -> bool {
     
     
    let t = ((entered - 0.04 - index as f64 * 0.055) / 0.42).clamp(0.0, 1.0) as f32;
    if t <= 0.0 {
        ui.ctx().request_repaint();
        return false;
    }
    let appear = ease_out_back(t);
    let fade = ease_out_cubic((t / 0.65).clamp(0.0, 1.0));

     
     
    let id = Id::new(("infered_menu_tile", index));
    let response = ui.interact(slot, id, Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(id.with("hover"), response.hovered(), 0.16);
    let press = ui.ctx().animate_bool_with_time(
        id.with("press"),
        response.is_pointer_button_down_on(),
        0.09,
    );

    let rect = slot
        .translate(vec2(-18.0 * (1.0 - appear), 14.0 * (1.0 - appear)))
        .translate(vec2(0.0, -2.5 * hover + 1.5 * press));
    let mut painter = ui.painter().clone();
    painter.set_opacity(fade);

     
    painter.rect_filled(
        rect.translate(vec2(0.0, 3.0)).expand(1.0),
        Rounding::same(16.0),
        with_alpha(Color32::BLACK, 0.32 + 0.12 * hover),
    );
    if hover > 0.01 {
        for (spread, alpha) in [(6.0_f32, 0.045_f32), (2.0, 0.07)] {
            painter.rect_filled(
                rect.expand(spread),
                Rounding::same(17.0),
                with_alpha(Palette::ACCENT, alpha * hover),
            );
        }
    }
    let body = lerp_color(Palette::CARD, Palette::CARD_HOVER, hover);
    painter.rect_filled(
        rect,
        Rounding::same(12.0),
        lerp_color(body, Palette::PANEL, 0.18 * press),
    );
     
    top_light_rounded(&painter, rect, 12.0, 0.07 + 0.03 * hover);
    if press > 0.01 {
        painter.rect_filled(
            rect,
            Rounding::same(12.0),
            with_alpha(Palette::ACCENT_BRIGHT, 0.05 * press),
        );
    }

     
    if hover > 0.01 {
        if let Some(pointer) = ui.ctx().pointer_hover_pos() {
            let focus = pos2(
                pointer.x.clamp(rect.left() + 10.0, rect.right() - 10.0),
                pointer.y.clamp(rect.top() + 8.0, rect.bottom() - 8.0),
            );
            radial_glow(
                &painter.with_clip_rect(rect.shrink(1.0)),
                focus,
                92.0,
                Palette::ACCENT,
                0.07 * hover,
            );
        }
    }

     
    painter.rect_stroke(
        rect,
        Rounding::same(12.0),
        Stroke::new(
            1.0_f32,
            with_alpha(
                if hover > 0.01 {
                    Palette::ACCENT
                } else {
                    Palette::TEXT_DIM
                },
                0.10 + 0.30 * hover + 0.18 * press,
            ),
        ),
    );
     
    painter.rect_filled(
        Rect::from_center_size(
            pos2(rect.left() + 1.5, rect.center().y),
            vec2(2.5, 18.0 + 26.0 * hover),
        ),
        Rounding::same(1.5),
        with_alpha(Palette::ACCENT, 0.12 + 0.70 * hover),
    );

     
     
     
     
    let title_w = tracked_width(&painter, entry.title, 12.5);
    let detail = elided(
        ui,
        entry.detail,
        10.5,
        with_alpha(Palette::TEXT_DIM, 0.62 + 0.20 * hover),
        (rect.width() - 2.0 * 14.0 - WELL - ICON_GAP).max(20.0),
    );
    let (well_centre, text_x) = tile_layout(rect, title_w.max(detail.size().x));

     
    let well = Rect::from_center_size(well_centre, Vec2::splat(WELL * (1.0 + 0.02 * hover)));
    painter.rect_filled(
        well,
        Rounding::same(12.0),
        with_alpha(Palette::ACCENT, 0.055 + 0.05 * hover),
    );
    radial_glow(
        &painter,
        well.center(),
        24.0 + 8.0 * hover,
        Palette::ACCENT,
        0.045 + 0.055 * hover,
    );
    painter.rect_stroke(
        well,
        Rounding::same(12.0),
        Stroke::new(1.0_f32, with_alpha(Palette::ACCENT, 0.16 + 0.28 * hover)),
    );
    paint_icon(
        &painter,
        entry.icon,
        well.center(),
        well.height() / 44.0 * 1.45,
        now,
        hover,
    );

     
    tracked(
        &painter,
        entry.title,
        12.5,
        lerp_color(with_alpha(Palette::TEXT, 0.86), Palette::TEXT, hover),
        Align::Min,
        text_x,
        rect.center().y - 10.0,
    );
    painter.galley(
        pos2(text_x, rect.center().y + 9.0 - detail.size().y * 0.5),
        detail,
        Palette::TEXT_DIM,
    );

     
     
    let corner = pos2(rect.right() - 13.0, rect.top() + 13.0);
    if hover < 0.999 {
        tracked(
            &painter,
            &format!("{:02}", index + 1),
            7.0,
            with_alpha(Palette::TEXT_DIM, 0.38 * (1.0 - hover)),
            Align::Max,
            corner.x,
            corner.y,
        );
    }
    if hover > 0.01 {
        tracked(
            &painter,
            "OPEN",
            7.0,
            with_alpha(Palette::ACCENT_BRIGHT, 0.95 * hover),
            Align::Max,
            corner.x,
            corner.y,
        );
    }

     
    super::widgets::sweep(
        &painter,
        rect.shrink(1.0),
        now + index as f64 * 1.7,
        7.5,
        0.024,
    );

    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response.clicked()
}

 

fn paint_icon(painter: &Painter, icon: Icon, centre: Pos2, scale: f32, now: f64, hover: f32) {
    match icon {
        Icon::Sliders => sliders(painter, centre, scale, now, hover),
        Icon::People => people(painter, centre, scale, now, hover),
        Icon::Play => play(painter, centre, scale, now, hover),
        Icon::Tiles => tiles(painter, centre, scale, now, hover),
    }
}

 
 
fn sliders(painter: &Painter, centre: Pos2, scale: f32, now: f64, hover: f32) {
     
    let rows = [
        (-6.4_f32, -4.2_f32, 4.4_f32, false),
        (0.0, 3.0, -3.4, true),
        (6.4, -1.2, 5.0, false),
    ];
    for (i, (y, rest, live, accent)) in rows.iter().enumerate() {
        let y = centre.y + y * scale;
        let left = centre.x - 8.0 * scale;
        let right = centre.x + 8.0 * scale;
        let wobble = 0.45 * (now as f32 * 1.6 + i as f32 * 1.4).sin() * (1.0 - hover) * scale;
        let knob = centre.x + (rest + (live - rest) * hover) * scale + wobble;

        painter.line_segment(
            [pos2(left, y), pos2(right, y)],
            Stroke::new(1.2_f32 * scale, with_alpha(Palette::TEXT_DIM, 0.28)),
        );
        painter.line_segment(
            [pos2(left, y), pos2(knob, y)],
            Stroke::new(
                1.6_f32 * scale,
                with_alpha(
                    if *accent {
                        Palette::ACCENT
                    } else {
                        Palette::TEXT_DIM
                    },
                    if *accent {
                        0.55 + 0.35 * hover
                    } else {
                        0.28 + 0.26 * hover
                    },
                ),
            ),
        );
        painter.circle_filled(
            pos2(knob, y),
            2.4 * scale,
            with_alpha(
                if *accent {
                    Palette::ACCENT_BRIGHT
                } else {
                    Palette::TEXT
                },
                0.78 + 0.22 * hover,
            ),
        );
        painter.circle_stroke(
            pos2(knob, y),
            2.4 * scale,
            Stroke::new(0.8_f32 * scale, with_alpha(Color32::BLACK, 0.35)),
        );
    }
}

 
 
fn people(painter: &Painter, centre: Pos2, scale: f32, now: f64, hover: f32) {
    painter.circle_filled(
        centre + vec2(-4.8, -3.6) * scale,
        2.9 * scale,
        with_alpha(Palette::TEXT_DIM, 0.55),
    );
    dome(
        painter,
        Rect::from_center_size(centre + vec2(-4.8, 4.8) * scale, vec2(9.2, 7.4) * scale),
        with_alpha(Palette::TEXT_DIM, 0.42),
    );

    let lean = -1.3 * hover;
    painter.circle_filled(
        centre + vec2(4.4 + lean, -2.4) * scale,
        3.2 * scale,
        with_alpha(Palette::ACCENT, 0.55 + 0.35 * hover),
    );
    dome(
        painter,
        Rect::from_center_size(
            centre + vec2(4.4 + lean, 6.4) * scale,
            vec2(10.4, 8.0) * scale,
        ),
        with_alpha(Palette::ACCENT, 0.38 + 0.34 * hover),
    );

     
     
    let twinkle = pulse(now, 1.8);
    let radius = (0.9 + 1.2 * twinkle + 1.1 * hover) * scale;
    let spark = centre + vec2(-0.2, -7.4) * scale;
    painter.add(Shape::convex_polygon(
        vec![
            spark + vec2(0.0, -radius),
            spark + vec2(radius, 0.0),
            spark + vec2(0.0, radius),
            spark + vec2(-radius, 0.0),
        ],
        with_alpha(Palette::ACCENT_BRIGHT, 0.30 + 0.55 * hover * twinkle),
        Stroke::new(0.0_f32, Color32::TRANSPARENT),
    ));
}

 
 
fn play(painter: &Painter, centre: Pos2, scale: f32, now: f64, hover: f32) {
    let radius = 8.6 * scale;
    painter.circle_stroke(
        centre,
        radius,
        Stroke::new(1.2_f32 * scale, with_alpha(Palette::TEXT_DIM, 0.24)),
    );

    let start = (now * 0.85) as f32;
    let sweep = 1.7 + 0.5 * hover;
    let arc: Vec<Pos2> = (0..=18)
        .map(|i| {
            let angle = start + sweep * i as f32 / 18.0;
            centre + vec2(angle.cos(), angle.sin()) * radius
        })
        .collect();
    painter.add(Shape::line(
        arc,
        Stroke::new(
            1.6_f32 * scale,
            with_alpha(Palette::ACCENT, 0.55 + 0.40 * hover),
        ),
    ));
    painter.circle_filled(
        centre + vec2((start + sweep).cos(), (start + sweep).sin()) * radius,
        1.6 * scale,
        Palette::ACCENT_BRIGHT,
    );

    let nudge = 0.9 * hover;
    painter.add(Shape::convex_polygon(
        vec![
            centre + vec2(-3.4 + nudge, -5.2) * scale,
            centre + vec2(-3.4 + nudge, 5.2) * scale,
            centre + vec2(4.6 + nudge, 0.0) * scale,
        ],
        with_alpha(Palette::TEXT, 0.76 + 0.24 * hover),
        Stroke::new(0.0_f32, Color32::TRANSPARENT),
    ));
}

 
 
fn tiles(painter: &Painter, centre: Pos2, scale: f32, now: f64, hover: f32) {
    let side = 6.4 * scale;
    let gap = (2.2 + 1.2 * hover) * scale;
    let offset = side * 0.5 + gap * 0.5;
    for i in 0..4 {
        let sx = if i % 2 == 0 { -1.0 } else { 1.0 };
        let sy = if i < 2 { -1.0 } else { 1.0 };
        let breathe = 0.5 + 0.5 * (now as f32 * 2.2 - i as f32 * 0.9).sin();
        let grow = 1.0 + 0.05 * breathe * (0.3 + hover);
        let cell = Rect::from_center_size(
            centre + vec2(sx * offset, sy * offset),
            Vec2::splat(side * grow),
        );
        let rounding = Rounding::same(1.8 * scale);
        if i == 3 {
            if hover < 0.999 {
                painter.rect_filled(
                    cell,
                    rounding,
                    with_alpha(Palette::ACCENT, 0.85 * (1.0 - hover)),
                );
            }
            if hover > 0.01 {
                let radius = side * 0.74 * grow;
                let c = cell.center();
                painter.add(Shape::convex_polygon(
                    vec![
                        c + vec2(0.0, -radius),
                        c + vec2(radius, 0.0),
                        c + vec2(0.0, radius),
                        c + vec2(-radius, 0.0),
                    ],
                    with_alpha(Palette::ACCENT_BRIGHT, 0.95 * hover),
                    Stroke::new(0.0_f32, Color32::TRANSPARENT),
                ));
            }
        } else {
            painter.rect_filled(
                cell,
                rounding,
                with_alpha(Palette::TEXT, 0.40 + 0.22 * hover + 0.10 * breathe),
            );
        }
    }
}

 
fn dome(painter: &Painter, rect: Rect, color: Color32) {
    painter.rect_filled(
        rect,
        Rounding {
            nw: rect.width() * 0.5,
            ne: rect.width() * 0.5,
            sw: 1.0,
            se: 1.0,
        },
        color,
    );
}

 

 
fn hash01(seed: u32) -> f32 {
    let mut x = seed.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    x = ((x >> 16) ^ x).wrapping_mul(0x45d9f3b);
    x = ((x >> 16) ^ x).wrapping_mul(0x45d9f3b);
    x = (x >> 16) ^ x;
    x as f32 / u32::MAX as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::InjectorGui;
    use egui::{Context, Pos2, RawInput};

     
    fn painted(gui: &mut InjectorGui) -> Vec<String> {
        let ctx = Context::default();
        crate::gui::theme::apply(&ctx);
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(670.0, 278.0));
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
                    .show(ctx, |ui| page(gui, ui, 10.0));
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

    fn opened() -> InjectorGui {
        let mut gui = InjectorGui::stub_gui();
        gui.drawer_page = DrawerPage::Menu;
        gui.page_entered_at = -10.0;
        gui
    }

    #[test]
    fn the_page_offers_every_drawer_destination() {
        let mut gui = opened();
        let joined = painted(&mut gui).join("");
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
    }

     
     
     
    #[test]
    fn the_page_has_no_status_pill_or_summary_bar() {
        let mut gui = opened();
         
        gui.drawer_page = DrawerPage::Menu;
        let joined = painted(&mut gui).join("|");
        for absent in ["READY", "NO CONFIG", "NO LIVE CONFIG", "MODULES", "SAVED"] {
            assert!(
                !joined.contains(absent),
                "{absent:?} is still painted: {joined:?}"
            );
        }
    }

     
     
    #[test]
    fn hovering_a_tile_reveals_its_open_tag() {
        let ctx = Context::default();
        crate::gui::theme::apply(&ctx);
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(670.0, 278.0));
        let mut gui = opened();
        let _ = ctx.run(
            RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |_| {},
        );

        let pointer = egui::pos2(160.0, 96.0);
        let mut shapes = Vec::new();
        for frame in 0..16 {
            let time = frame as f64 * 0.05;
            let output = ctx.run(
                RawInput {
                    screen_rect: Some(screen),
                    time: Some(time),
                    events: vec![egui::Event::PointerMoved(pointer)],
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default()
                        .frame(egui::Frame::none().fill(Palette::BG))
                        .show(ctx, |ui| page(&mut gui, ui, time));
                },
            );
            shapes = output.shapes;
        }

        let joined: String = shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");
        assert!(
            joined.contains("OPEN"),
            "hovering the first tile must reveal its OPEN tag: {joined:?}"
        );
    }

     
     
    #[test]
    fn a_tile_centres_its_icon_and_copy() {
        let rect = Rect::from_min_size(pos2(40.0, 100.0), vec2(325.0, 78.0));
        for copy_w in [40.0_f32, 120.0, 181.0] {
            let (well, text_x) = tile_layout(rect, copy_w);
            let left = (well.x - WELL * 0.5) - rect.left();
            let right = rect.right() - (text_x + copy_w);
            assert!(
                (left - right).abs() < 0.01,
                "copy {copy_w} leaves {left} on the left and {right} on the right"
            );
            assert!(
                (well.y - rect.center().y).abs() < 0.01,
                "vertically centred"
            );
            assert!(text_x > well.x, "the copy sits to the right of the well");
        }
    }

    #[test]
    fn the_dust_seeds_stay_inside_the_unit_interval() {
        for seed in 0..64 {
            let value = hash01(seed);
            assert!((0.0..=1.0).contains(&value), "seed {seed} gave {value}");
        }
        assert_ne!(hash01(1), hash01(2));
    }
}
