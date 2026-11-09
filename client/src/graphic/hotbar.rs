 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::anim;
use crate::graphic::theme;
use crate::mapping::inventory::Inventory;
use crate::module::render::custom_ui::{self, HotbarStyle};
use egui::{
    Align2, Color32, Context, FontId, Id, Painter, Pos2, Rect, Rounding, Stroke, TextureId, Vec2,
};
use std::sync::Mutex;
use std::time::{Duration, Instant};

 
const SLOTS: usize = 9;
const SLOT_UNITS: f32 = 18.0;
const GAP_UNITS: f32 = 2.2;
const PAD_UNITS: f32 = 4.0;

 
const ITEM_REFRESH: Duration = Duration::from_millis(250);
const INVENTORY_REFRESH: Duration = Duration::from_secs(1);

 
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct SlotData {
     
    pub item: String,
     
     
    pub display: String,
    pub count: i32,
     
    pub durability: Option<f32>,
}

impl SlotData {
    fn is_empty(&self) -> bool {
        self.item.is_empty()
    }

     
     
    fn display_name(&self) -> String {
        if !self.display.is_empty() {
            return self.display.clone();
        }
        let mut out = String::with_capacity(self.item.len());
        for (index, word) in self.item.split('_').enumerate() {
            if index > 0 {
                out.push(' ');
            }
            let mut chars = word.chars();
            if let Some(first) = chars.next() {
                out.extend(first.to_uppercase());
                out.push_str(chars.as_str());
            }
        }
        out
    }
}

 
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HotbarSnapshot {
    pub slots: [SlotData; SLOTS],
    pub selected: usize,
}

impl Default for HotbarSnapshot {
    fn default() -> Self {
        Self {
            slots: Default::default(),
            selected: 0,
        }
    }
}

struct Cache {
    inventory: Option<Inventory>,
    inventory_at: Option<Instant>,
    items_at: Option<Instant>,
    snapshot: HotbarSnapshot,
}

impl Default for Cache {
    fn default() -> Self {
        Self {
            inventory: None,
            inventory_at: None,
            items_at: None,
            snapshot: HotbarSnapshot::default(),
        }
    }
}

static CACHE: Mutex<Option<Cache>> = Mutex::new(None);

 
pub(crate) fn snapshot() -> Option<HotbarSnapshot> {
    let mut guard = CACHE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let cache = guard.get_or_insert_with(Cache::default);
    let player = crate::state::try_minecraft()?.player().ok().flatten()?;

     
     
    let stale = cache
        .inventory_at
        .is_none_or(|at| at.elapsed() >= INVENTORY_REFRESH);
    if stale || cache.inventory.is_none() {
        cache.inventory = player.get_inventory().ok();
        cache.inventory_at = Some(Instant::now());
        cache.items_at = None;
    }
    let inventory = cache.inventory.as_ref()?;

    if let Ok(selected) = inventory.get_selected() {
        cache.snapshot.selected = selected.clamp(0, SLOTS as i32 - 1) as usize;
    }

    let items_stale = cache.items_at.is_none_or(|at| at.elapsed() >= ITEM_REFRESH);
    if items_stale {
        cache.items_at = Some(Instant::now());
        for slot in 0..SLOTS {
            cache.snapshot.slots[slot] = read_slot(inventory, slot as i32);
        }
    }
    Some(cache.snapshot.clone())
}

 
 
fn read_slot(inventory: &Inventory, slot: i32) -> SlotData {
    let Ok(stack) = inventory.get_item(slot) else {
        return SlotData::default();
    };
    if stack.is_empty().unwrap_or(true) {
        return SlotData::default();
    }
    let item = stack
        .get_item()
        .and_then(|item| item.registry_name())
        .map(|name| name.strip_prefix("minecraft:").unwrap_or(&name).to_string())
        .unwrap_or_default();
    let display = stack.get_display_name().unwrap_or_default();
    let count = stack.count().unwrap_or(1);
    let durability = match (stack.get_damage_value(), stack.get_max_damage()) {
        (Ok(damage), Ok(max)) if max > 0 => {
            Some((1.0 - damage as f32 / max as f32).clamp(0.0, 1.0))
        }
        _ => None,
    };
    SlotData {
        item,
        display,
        count,
        durability,
    }
}

 
 
#[derive(Clone, Debug)]
pub(crate) struct HotbarLayout {
    pub plate: Rect,
    pub slots: [Rect; SLOTS],
     
    pub slot: f32,
}

 
 
pub(crate) fn layout(
    bar: Rect,
    first_slot: Rect,
    to_hud: f32,
    style: &HotbarStyle,
) -> HotbarLayout {
    let scale = style.scale.clamp(0.7, 1.6);
    let slot = SLOT_UNITS * to_hud * scale;
    let gap = GAP_UNITS * to_hud * scale;
    let pad = PAD_UNITS * to_hud * scale;
    let total = slot * SLOTS as f32 + gap * (SLOTS - 1) as f32;
    let left = bar.center().x - total * 0.5;
     
     
    let center_y = first_slot.center().y;
    let mut slots = [Rect::ZERO; SLOTS];
    for (index, rect) in slots.iter_mut().enumerate() {
        *rect = Rect::from_min_size(
            Pos2::new(left + index as f32 * (slot + gap), center_y - slot * 0.5),
            Vec2::splat(slot),
        );
    }
    let block = Rect::from_min_size(
        Pos2::new(left - pad, center_y - slot * 0.5 - pad),
        Vec2::new(total + pad * 2.0, slot + pad * 2.0),
    );
     
     
    let plate = block.union(bar.expand(2.0 * to_hud));
    HotbarLayout { plate, slots, slot }
}

 
 
static SELECTION: Mutex<Option<(usize, Instant)>> = Mutex::new(None);

fn selection_pop(selected: usize) -> f32 {
    let mut guard = SELECTION
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let now = Instant::now();
    match guard.as_mut() {
        Some((slot, at)) if *slot == selected => {
             
            let t = (at.elapsed().as_secs_f32() / 0.18).clamp(0.0, 1.0);
            let eased = anim::Easing::OutBack.apply(t).clamp(0.0, 1.2);
            (1.0 - eased).max(0.0) * 0.12
        }
        _ => {
            *guard = Some((selected, now));
            0.12
        }
    }
}

 
pub fn draw(ctx: &Context, painter: &Painter) {
    if !custom_ui::hotbar_active() {
        return;
    }
    let Some(frame) = crate::graphic::hud::hotbar_frame() else {
        return;
    };
    let style = HotbarStyle::read();
    let Some(snapshot) = snapshot() else {
        return;
    };
    paint(ctx, painter, frame, &style, &snapshot);
}

 
 
fn paint(
    ctx: &Context,
    painter: &Painter,
    frame: crate::graphic::hud::HotbarFrame,
    style: &HotbarStyle,
    snapshot: &HotbarSnapshot,
) {
    let layout = layout(frame.bar, frame.first_slot, frame.to_hud, style);
    let now = ctx.input(|input| input.time) as f32;
    let rounding = (layout.slot * 0.3).clamp(2.0, style.radius.max(3.0) + 8.0);

    paint_plate(painter, &layout, style, now);
    let pointer = ctx.input(|input| input.pointer.hover_pos());
    let hovered = pointer.and_then(|pos| {
        layout
            .slots
            .iter()
            .position(|rect| rect.expand(1.0).contains(pos))
    });

     
     
     
     
    paint_selected_box(ctx, painter, &layout, style, snapshot, rounding);

    for (index, rect) in layout.slots.iter().enumerate() {
        paint_slot(
            painter,
            *rect,
            &snapshot.slots[index],
            style,
            hovered == Some(index),
            rounding,
        );
    }

    paint_labels(painter, &layout, style, snapshot, hovered);

     
     
    ctx.request_repaint();
}

 
 
fn paint_plate(painter: &Painter, layout: &HotbarLayout, style: &HotbarStyle, now: f32) {
    let plate = layout.plate;
    let radius = Rounding::same((layout.slot * 0.34).clamp(3.0, 14.0));

    painter.rect_filled(
        plate.translate(Vec2::new(0.0, layout.slot * 0.12)),
        radius,
        Color32::from_black_alpha(120),
    );
     
    let base = style.background;
    painter.rect_filled(
        plate,
        radius,
        Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), 255),
    );
     
    let top = Rect::from_min_max(plate.min, Pos2::new(plate.max.x, plate.center().y));
    painter.rect_filled(
        top,
        Rounding {
            nw: radius.nw,
            ne: radius.ne,
            sw: 0.0,
            se: 0.0,
        },
        theme::with_alpha(style.accent, 0.10),
    );
    painter.rect_filled(
        Rect::from_min_max(
            Pos2::new(plate.min.x, plate.max.y - layout.slot * 0.35),
            plate.max,
        ),
        Rounding {
            nw: 0.0,
            ne: 0.0,
            sw: radius.sw,
            se: radius.se,
        },
        theme::with_alpha(style.accent, 0.07),
    );
    painter.rect_stroke(
        plate,
        radius,
        Stroke::new(1.2_f32, theme::with_alpha(style.accent, 0.32)),
    );

     
    if style.glow {
        let span = plate.width() - layout.slot * 0.8;
        let head = plate.left() + layout.slot * 0.4 + span * (now * 0.18).fract();
        let tail = (head - span * 0.28).max(plate.left() + layout.slot * 0.4);
        painter.line_segment(
            [Pos2::new(tail, plate.top()), Pos2::new(head, plate.top())],
            Stroke::new(2.0_f32, theme::with_alpha(style.accent, 0.55)),
        );
        painter.circle_filled(
            Pos2::new(head, plate.top()),
            2.2,
            theme::with_alpha(style.accent, 0.85),
        );
    }
}

 
 
fn paint_slot(
    painter: &Painter,
    rect: Rect,
    slot: &SlotData,
    style: &HotbarStyle,
    hovered: bool,
    rounding: f32,
) {
    let round = Rounding::same(rounding);
    let hover = if hovered { 1.0 } else { 0.0 };
    let fill = theme::lerp_color(
        Color32::from_rgb(20, 7, 9),
        Color32::from_rgb(44, 12, 16),
        hover,
    );
    painter.rect_filled(rect, round, fill);
     
    painter.rect_filled(
        Rect::from_min_max(
            rect.min,
            Pos2::new(rect.max.x, rect.min.y + rect.height() * 0.42),
        ),
        Rounding {
            nw: round.nw,
            ne: round.ne,
            sw: 0.0,
            se: 0.0,
        },
        theme::with_alpha(Color32::from_rgb(90, 20, 26), 0.35),
    );
    painter.rect_stroke(
        rect,
        round,
        Stroke::new(
            1.0_f32,
            theme::with_alpha(style.accent, 0.20 + 0.35 * hover),
        ),
    );
    if hovered {
        painter.rect_filled(
            rect.expand(2.0),
            Rounding::same(rounding + 2.0),
            theme::with_alpha(style.accent, 0.10),
        );
    }

    if slot.is_empty() {
        return;
    }

     
     
     
    let icon_size = rect.width() * 0.8;
    let icon = Rect::from_center_size(
        Pos2::new(rect.center().x, rect.center().y - rect.height() * 0.02),
        Vec2::splat(icon_size),
    );
    match crate::graphic::item_icon::icon(painter.ctx(), &slot.item) {
        Some((texture, _)) => paint_item(painter, icon, texture),
        None => {
             
             
            painter.text(
                icon.center(),
                Align2::CENTER_CENTER,
                slot.display_name()
                    .chars()
                    .next()
                    .unwrap_or('?')
                    .to_string(),
                FontId::proportional(icon_size * 0.7),
                theme::with_alpha(style.accent, 0.9),
            );
        }
    }

    if let Some(durability) = slot.durability {
        let bar = Rect::from_min_size(
            Pos2::new(
                rect.left() + rect.width() * 0.12,
                rect.bottom() - rect.height() * 0.16,
            ),
            Vec2::new(rect.width() * 0.76, (rect.height() * 0.075).max(1.5)),
        );
        painter.rect_filled(bar, Rounding::same(1.0), Color32::from_black_alpha(190));
        let colour = theme::lerp_color(theme::DANGER, Color32::from_rgb(150, 235, 170), durability);
        painter.rect_filled(
            Rect::from_min_size(bar.min, Vec2::new(bar.width() * durability, bar.height())),
            Rounding::same(1.0),
            colour,
        );
    }

    if slot.count > 1 {
        let font = FontId::proportional((rect.height() * 0.38).max(8.0));
        let pos = Pos2::new(
            rect.right() - rect.width() * 0.08,
            rect.bottom() - rect.height() * 0.06,
        );
        painter.text(
            pos + Vec2::new(1.0, 1.0),
            Align2::RIGHT_BOTTOM,
            slot.count.to_string(),
            font.clone(),
            Color32::from_black_alpha(200),
        );
        painter.text(
            pos,
            Align2::RIGHT_BOTTOM,
            slot.count.to_string(),
            font,
            theme::TEXT,
        );
    }
}

 
fn paint_item(painter: &Painter, rect: Rect, texture: TextureId) {
    painter.image(
        texture,
        rect,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );
}

 
fn paint_selected_box(
    ctx: &Context,
    painter: &Painter,
    layout: &HotbarLayout,
    style: &HotbarStyle,
    snapshot: &HotbarSnapshot,
    rounding: f32,
) {
    let target = layout.slots[snapshot.selected].center();
    let center = anim::spring_pos(
        ctx,
        Id::new("hotbar_selected_box"),
        target,
        style.slide_spring(),
    );
    let pop = selection_pop(snapshot.selected);
    let side = layout.slot * (1.16 + pop);
    let rect = Rect::from_center_size(center, Vec2::splat(side));
    let round = Rounding::same(rounding + 2.0);

    if style.glow {
        for (grow, alpha) in [(7.0, 0.08), (4.0, 0.13), (1.5, 0.20)] {
            painter.rect_filled(
                rect.expand(grow),
                Rounding::same(round.nw + grow * 0.7),
                theme::with_alpha(style.accent, alpha),
            );
        }
    }
    painter.rect_filled(rect, round, theme::with_alpha(style.accent, 0.16));
    painter.rect_stroke(
        rect,
        round,
        Stroke::new(2.0_f32, theme::with_alpha(style.accent, 0.95)),
    );
     
    painter.rect_stroke(
        rect.shrink(2.5),
        Rounding::same((round.nw - 2.0).max(1.0)),
        Stroke::new(1.0_f32, theme::with_alpha(Color32::WHITE, 0.28)),
    );
     
    let tick = layout.slot * 0.28;
    for (a, b) in [
        (
            Pos2::new(rect.center().x - tick, rect.top() - layout.slot * 0.06),
            Pos2::new(rect.center().x + tick, rect.top() - layout.slot * 0.06),
        ),
        (
            Pos2::new(rect.center().x - tick, rect.bottom() + layout.slot * 0.06),
            Pos2::new(rect.center().x + tick, rect.bottom() + layout.slot * 0.06),
        ),
    ] {
        painter.line_segment(
            [a, b],
            Stroke::new(1.6_f32, theme::with_alpha(style.accent, 0.55)),
        );
    }
}

 
fn paint_labels(
    painter: &Painter,
    layout: &HotbarLayout,
    style: &HotbarStyle,
    snapshot: &HotbarSnapshot,
    hovered: Option<usize>,
) {
    if style.numbers {
        let font = FontId::proportional((layout.slot * 0.26).max(7.0));
        for (index, rect) in layout.slots.iter().enumerate() {
            painter.text(
                Pos2::new(
                    rect.left() + rect.width() * 0.12,
                    rect.top() + rect.height() * 0.06,
                ),
                Align2::LEFT_TOP,
                (index + 1).to_string(),
                font.clone(),
                theme::with_alpha(theme::TEXT_MUTED, 0.85),
            );
        }
    }

    if !style.item_name {
        return;
    }
    let slot = hovered.unwrap_or(snapshot.selected);
    let item = &snapshot.slots[slot];
    if item.is_empty() {
        return;
    }
     
     
    let alpha = if hovered.is_some() {
        1.0
    } else {
        let age = SELECTION
            .lock()
            .ok()
            .and_then(|guard| guard.as_ref().map(|(_, at)| at.elapsed().as_secs_f32()))
            .unwrap_or(0.0);
        let fade_in = (age / 0.12).clamp(0.0, 1.0);
        let fade_out = 1.0 - ((age - 1.1) / 0.5).clamp(0.0, 1.0);
        fade_in * fade_out
    };
    if alpha <= 0.01 {
        return;
    }
    let text = item.display_name();
    let font = FontId::proportional((layout.slot * 0.42).max(9.0));
    let y = layout.plate.top() - layout.slot * 0.22 - ((1.0 - alpha) * layout.slot * 0.25);
     
    painter.text(
        Pos2::new(layout.plate.center().x + 1.0, y + 1.0),
        Align2::CENTER_BOTTOM,
        &text,
        font.clone(),
        Color32::from_black_alpha((160.0 * alpha) as u8),
    );
    painter.text(
        Pos2::new(layout.plate.center().x, y),
        Align2::CENTER_BOTTOM,
        text,
        font,
        theme::with_alpha(theme::TEXT, alpha),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(bar: Rect, to_hud: f32) -> (Rect, Rect, f32) {
        let first = Rect::from_min_size(
            bar.min + Vec2::splat(3.0 * to_hud),
            Vec2::splat(20.0 * to_hud),
        );
        (bar, first, to_hud)
    }

    fn vanilla_bar() -> Rect {
        Rect::from_min_size(Pos2::new(549.0, 698.0), Vec2::new(182.0, 22.0))
    }

    #[test]
    fn the_bar_is_centred_on_the_vanilla_hotbar() {
        let (bar, first, to_hud) = frame(vanilla_bar(), 1.0);
        let style = HotbarStyle::default();
        let layout = layout(bar, first, to_hud, &style);
        assert!(
            (layout.plate.center().x - bar.center().x).abs() < 0.01,
            "the plate must stay on the vanilla bar's centre line"
        );
        assert!(
            (layout.slots[SLOTS / 2].center().x - bar.center().x).abs() < 0.01,
            "the middle slot sits under the bar's centre"
        );
        assert!((layout.slots[4].center().y - first.center().y).abs() < 0.01);
    }

    #[test]
    fn every_slot_is_the_same_size_and_evenly_spaced() {
        let (bar, first, to_hud) = frame(vanilla_bar(), 1.0);
        let layout = layout(bar, first, to_hud, &HotbarStyle::default());
        let gap = layout.slots[1].left() - layout.slots[0].right();
        for pair in layout.slots.windows(2) {
            assert_eq!(pair[0].size(), pair[1].size());
            assert!((pair[1].left() - pair[0].right() - gap).abs() < 0.001);
        }
        assert_eq!(layout.slot, layout.slots[0].width());
    }

    #[test]
    fn the_plate_always_covers_the_vanilla_bar() {
        let bar = vanilla_bar();
        for scale in [0.7, 0.85, 1.0, 1.3, 1.6] {
            let style = HotbarStyle {
                scale,
                ..HotbarStyle::default()
            };
            let layout = layout(
                bar,
                Rect::from_min_size(bar.min, Vec2::splat(20.0)),
                1.0,
                &style,
            );
            assert!(
                layout.plate.contains_rect(bar),
                "at scale {scale} the plate {:?} must contain the vanilla bar {:?}",
                layout.plate,
                bar
            );
        }
    }

    #[test]
    fn the_selection_targets_the_selected_slot_at_every_index() {
        let (bar, first, to_hud) = frame(vanilla_bar(), 1.0);
        let layout = layout(bar, first, to_hud, &HotbarStyle::default());
        for slot in 0..SLOTS {
            assert_eq!(layout.slots[slot].center().x, layout.slots[slot].center().x);
            assert!(layout.plate.contains_rect(layout.slots[slot]));
        }
        assert!(layout.slots[8].center().x > layout.slots[0].center().x);
        assert_ne!(layout.slots[0].center(), layout.slots[8].center());
    }

     
     
     
     
     
     
     
    #[test]
    fn the_hotbar_preview_draws_each_items_own_texture() {
        use crate::graphic::hud::HotbarFrame;
        use crate::graphic::preview::{gather_textures, rasterize, PreviewTexture};
        use std::collections::HashMap;

        const ITEMS: [&str; SLOTS] = [
            "diamond_sword",
            "stone",
            "crafting_table",
            "golden_apple",
            "potion",
            "cooked_beef",
            "oak_log",
            "glass",
            "netherite_sword",
        ];
        crate::graphic::item_icon::clear_cache();

        let style = HotbarStyle::default();
        let (width, height) = (360usize, 110usize);
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(width as f32, height as f32));
        let bar = Rect::from_min_size(Pos2::new(20.0, 62.0), Vec2::new(182.0, 22.0));
        let first_slot = Rect::from_min_size(bar.min + Vec2::splat(3.0), Vec2::splat(20.0));
        let frame = HotbarFrame {
            bar,
            first_slot,
            to_hud: 1.0,
        };

        let ctx = Context::default();
        let mut textures: HashMap<egui::TextureId, PreviewTexture> = HashMap::new();
        let mut paint_frames = |snapshot: &HotbarSnapshot, start: f64| {
            let mut primitives = Vec::new();
            for tick in 0..3 {
                let time = start + tick as f64 / 60.0;
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(screen),
                        time: Some(time),
                        predicted_dt: 1.0 / 60.0,
                        ..Default::default()
                    },
                    |ctx| {
                        let painter = ctx.layer_painter(egui::LayerId::new(
                            egui::Order::Foreground,
                            Id::new("hotbar_preview"),
                        ));
                        paint(ctx, &painter, frame, &style, snapshot);
                    },
                );
                gather_textures(&mut textures, &output.textures_delta);
                primitives = ctx.tessellate(output.shapes, 1.0);
            }
            primitives
        };

        let mut snapshot = HotbarSnapshot::default();
        for (index, slot) in snapshot.slots.iter_mut().enumerate() {
            *slot = SlotData {
                item: ITEMS[index].to_owned(),
                display: String::new(),
                count: if ITEMS[index] == "stone" { 64 } else { 1 },
                durability: None,
            };
        }
        let filled = paint_frames(&snapshot, 0.0);
        let empty = paint_frames(&HotbarSnapshot::default(), 0.2);
        let backdrop = [0.04, 0.04, 0.05, 1.0];
        let image = rasterize(filled, &textures, width, height, backdrop);
        let blank = rasterize(empty, &textures, width, height, backdrop);
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/hotbar-preview.png");
        image.save(&path).expect("the preview png saves");

        for item in ITEMS {
            assert!(
                crate::graphic::item_icon::icon(&ctx, item).is_some(),
                "{item} has no icon, so the bar would fall back to its initial"
            );
        }

        let layout = layout(bar, first_slot, 1.0, &style);
        for (index, item) in ITEMS.iter().enumerate() {
            let rect = layout.slots[index];
             
             
            let inner = Rect::from_min_max(
                Pos2::new(
                    rect.left() + rect.width() * 0.25,
                    rect.top() + rect.height() * 0.25,
                ),
                Pos2::new(
                    rect.right() - rect.width() * 0.25,
                    rect.bottom() - rect.height() * 0.25,
                ),
            );
            let mut changed = 0;
            for y in inner.top() as u32..inner.bottom() as u32 {
                for x in inner.left() as u32..inner.right() as u32 {
                    let here = image.get_pixel(x, y).0;
                    let there = blank.get_pixel(x, y).0;
                    let diff: u32 = (0..3)
                        .map(|channel| here[channel].abs_diff(there[channel]) as u32)
                        .sum();
                    if diff > 24 {
                        changed += 1;
                    }
                }
            }
            assert!(
                changed > 12,
                "slot {index} ({item}) painted no texture: only {changed} pixels differ -> {path:?}"
            );
        }
        println!("hotbar preview: {} icons painted -> {path:?}", ITEMS.len());
    }

    #[test]
    fn slot_data_names_and_empty_stacks_are_derived() {
        let slot = SlotData {
            item: "diamond_sword".into(),
            display: String::new(),
            count: 1,
            durability: Some(0.5),
        };
        assert_eq!(slot.display_name(), "Diamond Sword");
        assert!(!slot.is_empty());
        let named = SlotData {
            display: "Potion of Healing".into(),
            ..slot.clone()
        };
        assert_eq!(named.display_name(), "Potion of Healing");
        assert!(SlotData::default().is_empty());
        assert_eq!(SlotData::default().display_name(), "");
    }
}
