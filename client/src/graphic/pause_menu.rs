 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::anim::{self, Easing};
use crate::graphic::input::GUI_OPEN;
use crate::graphic::theme;
use crate::mapping::component::Component;
use crate::mapping::java::iterable::Iterable;
use crate::mapping::{FieldType, MinecraftClassType};
use crate::module::render::custom_ui::{PauseStyle, UiStyle};
use crate::state::{mapping, try_minecraft};
use egui::{
    Align2, Color32, Context, FontId, Id, LayerId, Order, Painter, Pos2, Rect, Rounding, Shape,
    Stroke, Vec2,
};
use jni::objects::{GlobalRef, JObject, JValue};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Mutex;
use std::time::Instant;

 
struct PauseButton {
     
    widget: GlobalRef,
    label: String,
     
    rect: [i32; 4],
}

 
struct PauseMenu {
    screen: GlobalRef,
     
    size: [i32; 2],
    buttons: Vec<PauseButton>,
     
     
     
    scans: u8,
     
    opened: Instant,
}

 
const MAX_SCANS: u8 = 3;

 
const OPEN_TIME: f32 = 0.22;

static MENU: Mutex<Option<PauseMenu>> = Mutex::new(None);

 
 
 
static ACTIVE: AtomicBool = AtomicBool::new(false);

 
 
static PRESSED_ON: AtomicI32 = AtomicI32::new(-1);

 
static PENDING: AtomicI32 = AtomicI32::new(-1);

 
pub fn wants_input() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

 
 
 
pub fn update() {
     
     
     
    if let Some(pending) = take_pending() {
        let PendingPress { button, others } = pending;
         
         
         
         
         
         
        restore_buttons(&others);
        restore_buttons(std::slice::from_ref(&button));
        activate(&button);
        return;
    }

    if !crate::module::render::custom_ui::pause_menu_active() {
        stand_down();
        return;
    }

    let Some(minecraft) = try_minecraft() else {
        stand_down();
        return;
    };
     
     
     
     
     
    match minecraft.world() {
        Ok(Some(_)) => {}
        Ok(None) => {
            forget();
            return;
        }
         
         
        Err(error) => {
            log::debug!("Pause menu: could not read the open world: {error}");
            return;
        }
    }
     
     
    if minecraft.screen_is_null() {
        stand_down();
        return;
    }

    let screen = match minecraft.current_screen_object() {
        Ok(Some(screen)) => screen,
        Ok(None) => {
            stand_down();
            return;
        }
         
         
         
        Err(error) => {
            log::debug!("Pause menu: could not read the open screen: {error}");
            return;
        }
    };

    let is_pause = mapping()
        .is_instance_of(MinecraftClassType::PauseScreen, screen.as_obj())
        .unwrap_or(false);
    if !is_pause {
        stand_down();
        return;
    }

    let Ok(env) = mapping().get_env() else {
        return;
    };
    let mut guard = MENU.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

    let rebuild = match guard.as_mut() {
        Some(menu) => {
             
            !env
                .is_same_object(menu.screen.as_obj(), screen.as_obj())
                .unwrap_or(true)
                 
                 
                 
                || refresh(menu).is_err()
                     
                    || (menu.buttons.is_empty() && menu.scans < MAX_SCANS)
        }
        None => true,
    };
    if rebuild {
         
        let same_screen = guard.as_ref().is_some_and(|menu| {
            env.is_same_object(menu.screen.as_obj(), screen.as_obj())
                .unwrap_or(false)
        });
        let scans = match guard.as_ref() {
            Some(menu) if same_screen => menu.scans.saturating_add(1),
            _ => 1,
        };
        let previous = guard.take();
        match install(screen.as_obj(), scans) {
            Ok(menu) => *guard = Some(menu),
            Err(error) => {
                log::debug!("Pause menu: could not read the pause buttons: {error}");
                 
                 
                if let Some(previous) = previous {
                    restore_buttons(&previous.buttons);
                }
            }
        }
    }

    ACTIVE.store(
        guard.as_ref().is_some_and(|menu| !menu.buttons.is_empty()),
        Ordering::Relaxed,
    );
}

 
 
 
 
 
 
 
 
 
 
 
pub fn stand_down() {
    ACTIVE.store(false, Ordering::Relaxed);
    PRESSED_ON.store(-1, Ordering::Relaxed);
    PENDING.store(-1, Ordering::Relaxed);
    let menu = MENU
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take();
    if let Some(menu) = menu {
        if world_loaded() {
            restore_buttons(&menu.buttons);
        }
    }
}

 
 
 
 
 
 
fn forget() {
    ACTIVE.store(false, Ordering::Relaxed);
    PRESSED_ON.store(-1, Ordering::Relaxed);
    PENDING.store(-1, Ordering::Relaxed);
    *MENU.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
}

 
 
fn world_loaded() -> bool {
    try_minecraft().is_some_and(|minecraft| matches!(minecraft.world(), Ok(Some(_))))
}

 
 
 
fn restore_buttons(buttons: &[PauseButton]) {
    if buttons.is_empty() {
        return;
    }
    let _ = mapping().in_frame(|| {
        for button in buttons {
            set_visible(button.widget.as_obj(), true)?;
        }
        Ok(())
    });
}

 
 
struct PendingPress {
    button: PauseButton,
    others: Vec<PauseButton>,
}

 
 
fn take_pending() -> Option<PendingPress> {
    let index = PENDING.swap(-1, Ordering::Relaxed);
    if index < 0 {
        return None;
    }
    ACTIVE.store(false, Ordering::Relaxed);
    PRESSED_ON.store(-1, Ordering::Relaxed);
    let mut guard = MENU.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut menu = guard.take()?;
    let index = index as usize;
    if index >= menu.buttons.len() {
        return None;
    }
    let button = menu.buttons.swap_remove(index);
    Some(PendingPress {
        button,
        others: menu.buttons,
    })
}

 
 
fn install(screen: &JObject, scans: u8) -> anyhow::Result<PauseMenu> {
    mapping().in_frame(|| {
        let size = [screen_int(screen, "width")?, screen_int(screen, "height")?];

        let children = mapping()
            .call_method(MinecraftClassType::Screen, screen, "children", &[])?
            .l()?;
        let children = Iterable::new(mapping().new_global_ref(children)?);
        let iterator = children.iterator()?;

        let mut buttons = Vec::new();
        let result: anyhow::Result<()> = (|| {
            while iterator.has_next()? {
                let widget = iterator.next()?;
                if !mapping().is_instance_of(MinecraftClassType::Button, widget.as_obj())? {
                    continue;
                }
                 
                 
                let label = label_of(widget.as_obj())?;
                if label.trim().is_empty() {
                    continue;
                }
                let rect = widget_rect(widget.as_obj())?;
                 
                 
                 
                set_visible(widget.as_obj(), false)?;
                buttons.push(PauseButton {
                    rect,
                    widget,
                    label,
                });
            }
            Ok(())
        })();

         
         
        if result.is_err() {
            for button in &buttons {
                let _ = set_visible(button.widget.as_obj(), true);
            }
            buttons.clear();
        }
        result?;

        Ok(PauseMenu {
            screen: mapping().get_env()?.new_global_ref(screen)?,
            size,
            buttons,
            scans,
            opened: Instant::now(),
        })
    })
}

 
 
 
fn refresh(menu: &mut PauseMenu) -> anyhow::Result<()> {
    mapping().in_frame(|| {
        let size = [
            screen_int(menu.screen.as_obj(), "width")?,
            screen_int(menu.screen.as_obj(), "height")?,
        ];
        anyhow::ensure!(size == menu.size, "the pause screen was resized");
        for button in &mut menu.buttons {
            button.rect = widget_rect(button.widget.as_obj())?;
        }
        Ok(())
    })
}

 
 
fn screen_int(screen: &JObject, field: &str) -> anyhow::Result<i32> {
    Ok(mapping()
        .get_field(MinecraftClassType::Screen, screen, field, FieldType::Int)?
        .i()?)
}

 
fn widget_rect(widget: &JObject) -> anyhow::Result<[i32; 4]> {
    let int = |method: &str| -> anyhow::Result<i32> {
        Ok(mapping()
            .call_method(MinecraftClassType::AbstractWidget, widget, method, &[])?
            .i()?)
    };
    Ok([
        int("getX")?,
        int("getY")?,
        int("getWidth")?,
        int("getHeight")?,
    ])
}

 
fn label_of(widget: &JObject) -> anyhow::Result<String> {
    let message = mapping()
        .call_method(
            MinecraftClassType::AbstractWidget,
            widget,
            "getMessage",
            &[],
        )?
        .l()?;
    if message.is_null() {
        return Ok(String::new());
    }
    Component::new(mapping().new_global_ref(message)?).get_string()
}

 
fn set_visible(widget: &JObject, visible: bool) -> anyhow::Result<()> {
    mapping().set_field(
        MinecraftClassType::AbstractWidget,
        widget,
        "visible",
        FieldType::Boolean,
        JValue::Bool(visible.into()),
    )
}

 
 
fn activate(button: &PauseButton) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        mapping().call_method(
            MinecraftClassType::Button,
            button.widget.as_obj(),
            "onPress",
            &[],
        )
    }));
    match result {
        Ok(Ok(_)) => {}
        Ok(Err(error)) => log::warn!(
            "Pause menu: could not run the '{}' button: {error}",
            button.label
        ),
        Err(_) => log::warn!("Pause menu: the '{}' button panicked", button.label),
    }
     
     
    if let Ok(env) = mapping().get_env() {
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_describe();
            let _ = env.exception_clear();
        }
    }
}

 

 
 
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ButtonKind {
    Resume,
    Options,
    Advancements,
    Statistics,
    Lan,
    Quit,
    Other,
}

fn button_kind(label: &str) -> ButtonKind {
    let label = label.to_lowercase();
    if label.contains("back to game") || label.contains("resume") {
        ButtonKind::Resume
    } else if label.contains("option") || label.contains("settings") {
        ButtonKind::Options
    } else if label.contains("advancement") {
        ButtonKind::Advancements
    } else if label.contains("statistic") {
        ButtonKind::Statistics
    } else if label.contains("lan") {
        ButtonKind::Lan
    } else if label.contains("quit") || label.contains("disconnect") {
        ButtonKind::Quit
    } else {
        ButtonKind::Other
    }
}

 
#[derive(Clone, Debug)]
struct PauseLayout {
     
    card: Rect,
     
    buttons: Vec<Rect>,
}

 
 
fn pause_layout(viewport: Rect, size: [i32; 2], rects: &[Rect]) -> Option<PauseLayout> {
    if size[0] <= 0 || size[1] <= 0 || viewport.width() <= 1.0 || viewport.height() <= 1.0 {
        return None;
    }
    if rects.is_empty() {
        return None;
    }
     
     
     
    let single_column = rects
        .windows(2)
        .all(|pair| (pair[0].center().x - pair[1].center().x).abs() < 1.0);
    let buttons: Vec<Rect> = rects
        .iter()
        .map(|rect| {
            if single_column {
                let width = (rect.width() * 1.42).min(viewport.width() - 32.0);
                Rect::from_center_size(rect.center(), Vec2::new(width, rect.height()))
            } else {
                *rect
            }
        })
        .collect();

    let union = buttons.iter().copied().reduce(Rect::union)?;
    if !union.is_positive() {
        return None;
    }

     
     
     
     
     
    let unit = buttons.first().map_or(24.0, |rect| rect.height()).max(20.0);
    let pad_x = (union.width() * 0.12).clamp(18.0, 60.0);
    let header = (unit * 1.9).clamp(66.0, 132.0);
    let footer = (unit * 1.35).clamp(38.0, 84.0);
    let width = (union.width() + pad_x * 2.0).max(320.0);
    let height = union.height() + header + footer;
    let card = Rect::from_min_size(
        Pos2::new(union.center().x - width * 0.5, union.min.y - header),
        Vec2::new(width, height),
    );
    Some(PauseLayout {
        card: keep_on_screen(card, viewport, 14.0),
        buttons,
    })
}

 
 
fn keep_on_screen(rect: Rect, viewport: Rect, margin: f32) -> Rect {
    let mut center = rect.center();
    let (half_w, half_h) = (rect.width() * 0.5, rect.height() * 0.5);
    if rect.width() + margin * 2.0 > viewport.width() {
        center.x = viewport.center().x;
    } else {
        center.x = center.x.clamp(
            viewport.min.x + margin + half_w,
            viewport.max.x - margin - half_w,
        );
    }
    if rect.height() + margin * 2.0 > viewport.height() {
        center.y = viewport.center().y;
    } else {
        center.y = center.y.clamp(
            viewport.min.y + margin + half_h,
            viewport.max.y - margin - half_h,
        );
    }
    Rect::from_center_size(center, rect.size())
}

 
#[derive(Clone, Copy)]
struct PauseLook {
    style: UiStyle,
     
    pause: PauseStyle,
     
    reveal: f32,
     
    time: f64,
}

impl PauseLook {
    fn open(style: UiStyle, pause: PauseStyle, elapsed: f32, time: f64) -> Self {
        let reveal = if pause.animations {
            Easing::Out.apply((elapsed * pause.speed / OPEN_TIME).clamp(0.0, 1.0))
        } else {
            1.0
        };
        Self {
            style,
            pause,
            reveal,
            time,
        }
    }

     
    fn accent(&self, alpha: f32) -> Color32 {
        theme::with_alpha(self.style.accent, alpha)
    }

     
     
    fn fade(&self) -> f32 {
        0.25 + 0.75 * self.reveal
    }
}

 
 
 
fn posed_layout(layout: &PauseLayout, look: &PauseLook) -> PauseLayout {
    let eased = Easing::OutBack.apply(look.reveal).clamp(0.0, 1.05);
    let scale = 0.94 + 0.06 * eased;
    let offset = Vec2::new(0.0, (1.0 - eased) * 26.0);
    let center = layout.card.center();
    let pose = |rect: Rect| {
        Rect::from_center_size(
            center + (rect.center() - center) * scale + offset,
            rect.size() * scale,
        )
    };
    PauseLayout {
        card: pose(layout.card),
        buttons: layout.buttons.iter().map(|rect| pose(*rect)).collect(),
    }
}

 
pub fn draw(ctx: &Context) {
     
     
    if GUI_OPEN.load(Ordering::Relaxed) {
        return;
    }
     
     
     
     
    let style = UiStyle::read();
    let pause = PauseStyle::read();
    let guard = MENU.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(menu) = guard.as_ref() else {
        return;
    };
    if menu.buttons.is_empty() || menu.size[0] <= 0 || menu.size[1] <= 0 {
        return;
    }

    let viewport = ctx.screen_rect();
    let rects: Vec<Rect> = menu
        .buttons
        .iter()
        .map(|button| overlay_rect(viewport, menu.size, button.rect))
        .collect();
    let Some(layout) = pause_layout(viewport, menu.size, &rects) else {
        return;
    };

    let look = PauseLook::open(
        style,
        pause,
        menu.opened.elapsed().as_secs_f32(),
        ctx.input(|input| input.time),
    );
     
     
    let layout = posed_layout(&layout, &look);
    let painter = ctx.layer_painter(LayerId::new(
        Order::Foreground,
        Id::new("pause_menu_replacement"),
    ));

    paint_chrome(&painter, viewport, &layout, &look);

    let pointer = ctx.input(|input| input.pointer.interact_pos());
    let hovered = pointer.and_then(|pos| layout.buttons.iter().position(|rect| rect.contains(pos)));

     
     
    if ctx.input(|input| input.pointer.primary_pressed()) {
        PRESSED_ON.store(hovered.map_or(-1, |index| index as i32), Ordering::Relaxed);
    }
    let released_on = if ctx.input(|input| input.pointer.primary_released()) {
        match (PRESSED_ON.swap(-1, Ordering::Relaxed), hovered) {
            (pressed, Some(index)) if pressed == index as i32 => Some(index),
            _ => None,
        }
    } else {
        None
    };

    if hovered.is_some() {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    let primary_down = ctx.input(|input| input.pointer.primary_down());
    for (index, (button, rect)) in menu.buttons.iter().zip(layout.buttons.iter()).enumerate() {
        let id = Id::new("pause_menu_button")
            .with(index)
            .with(button.label.as_str());
        let hover = anim::toggle(
            ctx,
            id.with("hover"),
            hovered == Some(index),
            0.14,
            Easing::Out,
        );
        let press = anim::toggle(
            ctx,
            id.with("press"),
            primary_down && PRESSED_ON.load(Ordering::Relaxed) == index as i32,
            0.06,
            Easing::Out,
        );
         
        let reveal = ((look.reveal * 1.5) - index as f32 * 0.09).clamp(0.0, 1.0);
        paint_button(
            &painter,
            &ButtonArt {
                rect: *rect,
                label: &button.label,
                kind: button_kind(&button.label),
                hover,
                press,
                reveal,
            },
            &look,
        );
    }

    paint_footer(&painter, &layout, &look);

    if let Some(index) = released_on {
         
         
        PENDING.store(index as i32, Ordering::Relaxed);
        ctx.request_repaint();
    }
}

 
fn paint_chrome(painter: &Painter, viewport: Rect, layout: &PauseLayout, look: &PauseLook) {
    let style = &look.style;
    let fade = look.fade();

     
     
    painter.rect_filled(
        viewport,
        Rounding::ZERO,
        Color32::from_black_alpha((look.pause.dim * 255.0 * fade) as u8),
    );
    let tint = Color32::from_rgba_unmultiplied(
        style.accent.r(),
        style.accent.g(),
        style.accent.b(),
        (14.0 * fade) as u8,
    );
    let mut wash = egui::Mesh::default();
    wash.colored_vertex(viewport.left_top(), tint);
    wash.colored_vertex(viewport.right_top(), tint);
    wash.colored_vertex(
        viewport.right_bottom(),
        Color32::from_rgba_unmultiplied(style.accent.r(), style.accent.g(), style.accent.b(), 0),
    );
    wash.colored_vertex(
        viewport.left_bottom(),
        Color32::from_rgba_unmultiplied(style.accent.r(), style.accent.g(), style.accent.b(), 0),
    );
    wash.add_triangle(0, 1, 2);
    wash.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(wash));

     
     
     
    if look.pause.embers {
        paint_embers(painter, viewport, look);
    }
    paint_aurora(painter, layout, look);

    let card = layout.card;
    let radius = (style.radius + 4.0).clamp(9.0, 20.0);
    let rounding = Rounding::same(radius);

     
    for (grow, alpha) in [(10.0_f32, 12_u8), (5.0, 20), (1.5, 34)] {
        painter.rect_filled(
            card.translate(Vec2::new(0.0, 5.0)).expand(grow),
            Rounding::same(radius + grow * 0.6),
            Color32::from_black_alpha(alpha),
        );
    }

     
    if style.glow && style.glow_strength > 0.0 {
        let pulse = if style.glow_pulse {
            0.55 + 0.45 * ((look.time as f32 * style.pulse_speed * std::f32::consts::TAU).sin())
        } else {
            1.0
        };
        painter.rect_stroke(
            card.expand(2.0),
            Rounding::same(radius + 2.0),
            Stroke::new(
                2.0_f32,
                look.accent(0.10 * style.glow_strength * pulse * fade),
            ),
        );
    }

     
     
    let base_alpha = (style.background.a() as f32 / 255.0).max(0.93);
    let lifted = theme::lerp_color(style.background, theme::SURFACE, 0.35);
    let body = Color32::from_rgba_unmultiplied(
        lifted.r(),
        lifted.g(),
        lifted.b(),
        (base_alpha * 255.0) as u8,
    );
    painter.rect_filled(card, rounding, body);
     
    let sheen = Rect::from_min_max(
        card.min,
        Pos2::new(card.max.x, card.min.y + card.height() * 0.52),
    );
    painter.rect_filled(
        sheen,
        Rounding {
            nw: radius,
            ne: radius,
            sw: 0.0,
            se: 0.0,
        },
        Color32::from_rgba_unmultiplied(
            theme::SURFACE.r(),
            theme::SURFACE.g(),
            theme::SURFACE.b(),
            16,
        ),
    );
    painter.rect_stroke(
        card,
        rounding,
        Stroke::new(
            style.border_width.max(1.0),
            theme::with_alpha(theme::BORDER, 0.9 * fade),
        ),
    );

     
    let line_left = card.left() + radius;
    let line_right = card.right() - radius;
    painter.line_segment(
        [
            Pos2::new(line_left, card.top()),
            Pos2::new(line_right, card.top()),
        ],
        Stroke::new(1.0_f32, look.accent(0.32 * fade)),
    );
    if style.comets && line_right > line_left {
        let span = line_right - line_left;
        let phase = (look.time as f32 * style.comet_speed)
            .fract()
            .clamp(0.0, 1.0);
        let head = Pos2::new(line_left + span * phase, card.top());
        let tail = Pos2::new(
            (head.x - span * style.comet_tail.min(0.4)).max(line_left),
            head.y,
        );
        painter.line_segment([tail, head], Stroke::new(2.0_f32, look.accent(0.55 * fade)));
        painter.circle_filled(head, 4.5, look.accent(0.10 * fade));
        painter.circle_filled(head, 2.0, look.accent(0.95 * fade));
    }

     
     
     
     
    let unit = layout
        .buttons
        .first()
        .map_or(24.0, |rect| rect.height())
        .max(20.0);
    let pad = (unit * 0.62).clamp(16.0, 32.0);
     
     
     
    let header = (unit * 1.9).clamp(66.0, 132.0);
    let brand_mid = card.top() + header * 0.22;
    let mark = (unit * 0.24).clamp(4.0, 9.0);
    let mark_center = Pos2::new(card.left() + pad + mark, brand_mid);
     
     
    let spin = look.time as f32 * 0.9;
    let halo = 0.55 + 0.45 * ((look.time as f32 * 1.6).sin() * 0.5 + 0.5);
    painter.circle_filled(mark_center, mark * 1.9, look.accent(0.10 * fade * halo));
    let points: Vec<Pos2> = (0..4)
        .map(|i| {
            let angle = spin + std::f32::consts::FRAC_PI_2 * i as f32 - std::f32::consts::FRAC_PI_2;
            mark_center + Vec2::new(angle.cos(), angle.sin()) * mark
        })
        .collect();
    painter.add(Shape::convex_polygon(
        points,
        look.accent(0.92 * fade),
        Stroke::NONE,
    ));
    for i in 0..2 {
        let angle = spin * 1.7 + i as f32 * std::f32::consts::PI;
        let orbit = mark * 2.5;
        painter.circle_filled(
            mark_center + Vec2::new(angle.cos(), angle.sin()) * orbit,
            (mark * 0.22).max(1.0),
            look.accent(0.65 * fade),
        );
    }
    let brand = painter.layout_no_wrap(
        "INFERED".to_string(),
        FontId::proportional((unit * 0.26).clamp(9.0, 13.0)),
        look.accent(0.95 * fade),
    );
    let brand_pos = Pos2::new(
        mark_center.x + mark + unit * 0.3,
        brand_mid - brand.size().y * 0.5,
    );
    let rule_start = brand_pos.x + brand.size().x + unit * 0.45;
    painter.galley(brand_pos, brand, look.accent(0.95 * fade));
     
    let rule_end = rule_start + (card.right() - pad - rule_start) * look.reveal.clamp(0.0, 1.0);
    if rule_end > rule_start + 1.0 {
        painter.line_segment(
            [
                Pos2::new(rule_start, brand_mid),
                Pos2::new(rule_end, brand_mid),
            ],
            Stroke::new(1.0_f32, theme::with_alpha(theme::BORDER, 0.8 * fade)),
        );
    }

    let title_size = ((unit * 0.62).clamp(18.0, 34.0)).min(header * 0.42);
     
    let title_x = card.left() + pad - (1.0 - look.reveal) * unit * 0.7;
    painter.text(
        Pos2::new(title_x, card.top() + header * 0.55),
        Align2::LEFT_CENTER,
        "GAME PAUSED",
        FontId::proportional(title_size),
        theme::with_alpha(style.text, 0.97 * fade),
    );
    let rule_y = card.top() + header * 0.85;
    let rule_right = card.left() + pad + (card.width() - pad * 2.0) * look.reveal.clamp(0.0, 1.0);
    painter.line_segment(
        [
            Pos2::new(card.left() + pad, rule_y),
            Pos2::new(rule_right.max(card.left() + pad), rule_y),
        ],
        Stroke::new(1.0_f32, theme::with_alpha(theme::BORDER, 0.85 * fade)),
    );
     
    let underline = Rect::from_min_size(
        Pos2::new(title_x, card.top() + header * 0.68),
        Vec2::new(
            title_size * 1.9 * look.reveal.clamp(0.0, 1.0),
            2.0_f32.max(1.0),
        ),
    );
    painter.rect_filled(underline, Rounding::same(1.0), look.accent(0.85 * fade));
}

 
 
fn hash01(seed: u32) -> f32 {
    let mut x = seed.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    x = ((x >> ((x >> 28) + 4)) ^ x).wrapping_mul(277_803_737);
    x = (x >> 22) ^ x;
    x as f32 / u32::MAX as f32
}

 
 
fn paint_embers(painter: &Painter, viewport: Rect, look: &PauseLook) {
    let count = look.pause.ember_count.min(80);
    let accent = look.style.accent;
    let time = look.time as f32;
    for i in 0..count {
        let seed = i as u32 * 4;
        let lane = hash01(seed + 1);
        let speed = 0.025 + 0.075 * hash01(seed + 2);
        let phase = hash01(seed + 3);
        let life = (time * speed + phase).fract();
        let sway = (time * 0.6 + phase * 11.0).sin() * viewport.width() * 0.015;
        let x = viewport.left() + lane * viewport.width() + sway;
        let y = viewport.bottom() - life * viewport.height() * 1.08;
        let dot = (0.9 + 2.0 * hash01(seed + 4)) * look.style.scale.max(0.6);
         
        let alpha = (std::f32::consts::PI * life).sin().max(0.0) * 0.34;
        painter.circle_filled(Pos2::new(x, y), dot, theme::with_alpha(accent, alpha));
    }
    painter.ctx().request_repaint();
}

 
 
fn paint_aurora(painter: &Painter, layout: &PauseLayout, look: &PauseLook) {
    let card = layout.card;
    let accent = look.style.accent;
    let time = look.time as f32;
    for i in 0..3 {
        let angle = time * 0.22 + i as f32 * std::f32::consts::TAU / 3.0;
        let center = card.center()
            + Vec2::new(
                angle.cos() * card.width() * 0.55,
                angle.sin() * card.height() * 0.62,
            );
        for (radius, alpha) in [
            (card.width() * 0.40, 0.022),
            (card.width() * 0.25, 0.032),
            (card.width() * 0.13, 0.042),
        ] {
            painter.circle_filled(center, radius, theme::with_alpha(accent, alpha));
        }
    }
    if look.pause.animations {
        painter.ctx().request_repaint();
    }
}

 
struct ButtonArt<'a> {
    rect: Rect,
    label: &'a str,
    kind: ButtonKind,
    hover: f32,
    press: f32,
     
    reveal: f32,
}

 
 
fn paint_button(painter: &Painter, art: &ButtonArt, look: &PauseLook) {
    let style = &look.style;
    let eased = Easing::OutBack.apply(art.reveal).clamp(0.0, 1.0);
    let fade = art.reveal.clamp(0.0, 1.0) * look.fade();
    if fade <= 0.0 {
        return;
    }
    let rect = art
        .rect
        .shrink(1.0)
        .translate(Vec2::new(0.0, (1.0 - eased) * 12.0));
    let shape = Rect::from_center_size(rect.center(), rect.size() * (1.0 - 0.018 * art.press));
    let radius = Rounding::same((shape.height() * 0.26).clamp(5.0, 11.0));
    let primary = art.kind == ButtonKind::Resume;
    let destructive = art.kind == ButtonKind::Quit;
    let emphasis = if destructive {
        theme::DANGER
    } else {
        style.accent
    };
    let hover = art.hover.clamp(0.0, 1.0);

     
    if hover > 0.001 {
        painter.rect_filled(
            shape.expand(2.0 + 2.5 * hover),
            Rounding::same(radius.nw + 2.0),
            theme::with_alpha(emphasis, 0.10 * hover * fade),
        );
    }

     
    let mut fill = theme::lerp_color(theme::SURFACE, theme::SURFACE_HOVER, hover);
    if primary {
        fill = theme::lerp_color(fill, style.accent, 0.24 + 0.14 * hover);
    } else if destructive && hover > 0.0 {
        fill = theme::lerp_color(fill, theme::DANGER, 0.14 * hover);
    }
    let border = theme::lerp_color(
        theme::BORDER,
        emphasis,
        if primary {
            0.5 + 0.45 * hover
        } else {
            0.10 + 0.65 * hover
        },
    );
    painter.rect_filled(
        shape.translate(Vec2::new(0.0, 2.0 * (1.0 - art.press))),
        radius,
        Color32::from_black_alpha((55.0 * fade) as u8),
    );
    painter.rect(
        shape,
        radius,
        theme::with_alpha(fill, fade),
        Stroke::new(1.0_f32, theme::with_alpha(border, fade)),
    );

     
     
    if hover > 0.01 {
        let phase = (look.time as f32 * 0.65).fract();
        let center_x = shape.left() + (phase * 1.7 - 0.35) * shape.width();
        let sweep = Rect::from_center_size(
            Pos2::new(center_x, shape.center().y),
            Vec2::new(shape.width() * 0.18, shape.height() * 1.8),
        );
        let clipped = painter.with_clip_rect(shape);
        clipped.add(Shape::convex_polygon(
            vec![
                sweep.left_top() + Vec2::new(shape.width() * 0.06, 0.0),
                sweep.right_top(),
                sweep.right_bottom(),
                sweep.left_bottom() + Vec2::new(-shape.width() * 0.06, 0.0),
            ],
            theme::with_alpha(Color32::WHITE, 0.055 * hover * fade),
            Stroke::NONE,
        ));
        painter.ctx().request_repaint();
    }

    let height = shape.height();
     
    let nudge = height * 0.09 * hover;
     
    let bar = Rect::from_center_size(
        Pos2::new(shape.left() + height * 0.15, shape.center().y),
        Vec2::new(
            (height * 0.065).clamp(2.0, 4.0),
            height * (0.34 + 0.24 * hover + 0.12 * if primary { 1.0 } else { 0.0 }),
        ),
    );
    painter.rect_filled(
        bar,
        Rounding::same(1.2),
        theme::with_alpha(emphasis, (0.55 + 0.45 * hover) * fade),
    );

    let icon_size = (height * 0.26).clamp(7.0, 15.0);
    paint_icon(
        painter,
        art.kind,
        Pos2::new(shape.left() + height * 0.62 + nudge, shape.center().y),
        icon_size,
        theme::lerp_color(
            theme::TEXT_MUTED,
            emphasis,
            0.35 + 0.55 * hover.max(if primary { 0.5 } else { 0.0 }),
        ),
    );

    let font_size = (height * 0.36).clamp(12.0, 21.0);
    let text_color = theme::lerp_color(
        theme::TEXT_DIM,
        style.text,
        hover.max(if primary { 0.55 } else { 0.0 }),
    );
    painter.text(
        Pos2::new(shape.left() + height * 1.24 + nudge, shape.center().y),
        Align2::LEFT_CENTER,
        art.label,
        FontId::proportional(font_size),
        theme::with_alpha(text_color, fade),
    );

    if primary {
         
        let key = painter.layout_no_wrap(
            "ESC".to_string(),
            FontId::proportional((height * 0.2).clamp(8.0, 10.5)),
            theme::with_alpha(theme::TEXT_DIM, 0.95 * fade),
        );
        let key_h = (height * 0.42).clamp(12.0, 20.0);
        let key_rect = Rect::from_min_size(
            Pos2::new(
                shape.right() - height * 0.3 - (key.size().x + height * 0.34),
                shape.center().y - key_h * 0.5,
            ),
            Vec2::new(key.size().x + height * 0.34, key_h),
        );
        painter.rect_filled(
            key_rect,
            Rounding::same(3.0),
            theme::with_alpha(Color32::WHITE, 0.07 * fade),
        );
        painter.rect_stroke(
            key_rect,
            Rounding::same(3.0),
            Stroke::new(1.0_f32, theme::with_alpha(Color32::WHITE, 0.18 * fade)),
        );
        painter.galley(
            Pos2::new(
                key_rect.center().x - key.size().x * 0.5,
                key_rect.center().y - key.size().y * 0.5,
            ),
            key,
            Color32::PLACEHOLDER,
        );
    } else if matches!(
        art.kind,
        ButtonKind::Options | ButtonKind::Advancements | ButtonKind::Statistics | ButtonKind::Lan
    ) {
         
        let c = Pos2::new(shape.right() - height * 0.4, shape.center().y);
        let dot = (height * 0.09).clamp(2.5, 5.0);
        let stroke = Stroke::new(1.4_f32, theme::with_alpha(theme::TEXT_MUTED, 0.8 * fade));
        painter.line_segment(
            [
                Pos2::new(c.x - dot, c.y - dot * 1.3),
                Pos2::new(c.x + dot * 0.5, c.y),
            ],
            stroke,
        );
        painter.line_segment(
            [
                Pos2::new(c.x + dot * 0.5, c.y),
                Pos2::new(c.x - dot, c.y + dot * 1.3),
            ],
            stroke,
        );
    }
}

 
 
fn paint_footer(painter: &Painter, layout: &PauseLayout, look: &PauseLook) {
    let fade = look.fade() * ((look.reveal * 1.5 - 0.4).clamp(0.0, 1.0));
    if fade <= 0.0 {
        return;
    }
    let unit = layout
        .buttons
        .first()
        .map_or(24.0, |rect| rect.height())
        .max(20.0);
    let pad = (unit * 0.62).clamp(16.0, 32.0);
    let y = layout.card.bottom() - (unit * 0.55).clamp(16.0, 30.0);
    painter.line_segment(
        [
            Pos2::new(layout.card.left() + pad, y - unit * 0.32),
            Pos2::new(layout.card.right() - pad, y - unit * 0.32),
        ],
        Stroke::new(1.0_f32, theme::with_alpha(theme::BORDER, 0.7 * fade)),
    );

    let font = FontId::proportional((unit * 0.22).clamp(8.5, 11.5));
    let key_font = FontId::proportional((unit * 0.2).clamp(8.0, 10.5));
    let key_h = (unit * 0.42).clamp(13.0, 21.0);
    let hints = [("ESC", "resume"), ("RIGHT SHIFT", "client menu")];
    let text = theme::with_alpha(theme::TEXT_MUTED, 0.95 * fade);
    let key_text = theme::with_alpha(theme::TEXT_DIM, 0.95 * fade);
    let galleys: Vec<_> = hints
        .iter()
        .map(|(key, label)| {
            (
                painter.layout_no_wrap(key.to_string(), key_font.clone(), key_text),
                painter.layout_no_wrap(label.to_string(), font.clone(), text),
            )
        })
        .collect();
    let widths: Vec<f32> = galleys
        .iter()
        .map(|(key, label)| key.size().x + unit * 0.34 + unit * 0.22 + label.size().x)
        .collect();
    let gap = unit * 0.9;
    let total = widths.iter().sum::<f32>() + gap;
    let mut x = layout.card.center().x - total * 0.5;
    for ((key_galley, label_galley), width) in galleys.into_iter().zip(widths) {
        let key_rect = Rect::from_min_size(
            Pos2::new(x, y - key_h * 0.5),
            Vec2::new(key_galley.size().x + unit * 0.34, key_h),
        );
        painter.rect_filled(
            key_rect,
            Rounding::same(3.0),
            theme::with_alpha(theme::SURFACE_HOVER, 0.95 * fade),
        );
        painter.rect_stroke(
            key_rect,
            Rounding::same(3.0),
            Stroke::new(1.0_f32, theme::with_alpha(theme::BORDER, 0.9 * fade)),
        );
        painter.galley(
            Pos2::new(
                key_rect.center().x - key_galley.size().x * 0.5,
                key_rect.center().y - key_galley.size().y * 0.5,
            ),
            key_galley,
            Color32::PLACEHOLDER,
        );
        painter.galley(
            Pos2::new(
                key_rect.right() + unit * 0.22,
                y - label_galley.size().y * 0.5,
            ),
            label_galley,
            Color32::PLACEHOLDER,
        );
        x += width + gap;
    }
}

 
fn paint_icon(painter: &Painter, kind: ButtonKind, center: Pos2, size: f32, color: Color32) {
    let ink = Stroke::new(1.4_f32, color);
    let s = size;
    match kind {
        ButtonKind::Resume => {
            painter.add(Shape::convex_polygon(
                vec![
                    center + Vec2::new(-s * 0.45, -s * 0.8),
                    center + Vec2::new(s * 0.7, 0.0),
                    center + Vec2::new(-s * 0.45, s * 0.8),
                ],
                color,
                Stroke::NONE,
            ));
        }
        ButtonKind::Options => {
            for (dy, knob) in [(-s * 0.65, 0.45_f32), (0.0, -0.35), (s * 0.65, 0.15)] {
                let y = center.y + dy;
                painter.line_segment(
                    [Pos2::new(center.x - s, y), Pos2::new(center.x + s, y)],
                    Stroke::new(1.2_f32, color),
                );
                painter.circle_filled(Pos2::new(center.x + s * knob, y), 1.5, color);
            }
        }
        ButtonKind::Advancements => {
            painter.line_segment(
                [
                    Pos2::new(center.x - s * 0.55, center.y - s * 0.85),
                    Pos2::new(center.x - s * 0.55, center.y + s * 0.85),
                ],
                ink,
            );
            painter.add(Shape::convex_polygon(
                vec![
                    Pos2::new(center.x - s * 0.55, center.y - s * 0.8),
                    Pos2::new(center.x + s * 0.85, center.y - s * 0.45),
                    Pos2::new(center.x - s * 0.55, center.y - s * 0.05),
                ],
                color,
                Stroke::NONE,
            ));
        }
        ButtonKind::Statistics => {
            for (i, height) in [0.35_f32, 0.75, 0.55].into_iter().enumerate() {
                let x = center.x - s * 0.65 + i as f32 * s * 0.65;
                let bar = Rect::from_min_max(
                    Pos2::new(x, center.y + s * 0.8 - s * 1.6 * height),
                    Pos2::new(x + s * 0.34, center.y + s * 0.8),
                );
                painter.rect_filled(bar, Rounding::same(0.8), color);
            }
        }
        ButtonKind::Lan => {
            let left = Pos2::new(center.x - s * 0.75, center.y + s * 0.35);
            let right = Pos2::new(center.x + s * 0.75, center.y - s * 0.35);
            painter.line_segment([left, right], Stroke::new(1.2_f32, color));
            painter.circle_stroke(left, s * 0.28, ink);
            painter.circle_filled(right, s * 0.2, color);
        }
        ButtonKind::Quit => {
            painter.rect_stroke(
                Rect::from_min_max(
                    Pos2::new(center.x - s * 0.85, center.y - s * 0.85),
                    Pos2::new(center.x + s * 0.1, center.y + s * 0.85),
                ),
                Rounding::same(1.2),
                ink,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x - s * 0.05, center.y),
                    Pos2::new(center.x + s * 0.9, center.y),
                ],
                ink,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x + s * 0.9, center.y),
                    Pos2::new(center.x + s * 0.45, center.y - s * 0.38),
                ],
                ink,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x + s * 0.9, center.y),
                    Pos2::new(center.x + s * 0.45, center.y + s * 0.38),
                ],
                ink,
            );
        }
        ButtonKind::Other => {
            painter.add(Shape::convex_polygon(
                vec![
                    center + Vec2::new(0.0, -s * 0.65),
                    center + Vec2::new(s * 0.65, 0.0),
                    center + Vec2::new(0.0, s * 0.65),
                    center + Vec2::new(-s * 0.65, 0.0),
                ],
                Color32::TRANSPARENT,
                ink,
            ));
        }
    }
}

 
 
 
fn overlay_rect(viewport: Rect, size: [i32; 2], rect: [i32; 4]) -> Rect {
    let scale = Vec2::new(
        viewport.width() / size[0] as f32,
        viewport.height() / size[1] as f32,
    );
    Rect::from_min_size(
        Pos2::new(
            viewport.min.x + rect[0] as f32 * scale.x,
            viewport.min.y + rect[1] as f32 * scale.y,
        ),
        Vec2::new(rect[2] as f32 * scale.x, rect[3] as f32 * scale.y),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphic::preview::{gather_textures, rasterize};
    use egui::{pos2, Vec2};
    use std::collections::HashMap;

     
     
    #[test]
    fn button_rectangles_follow_the_overlay_scale() {
        let screen = Rect::from_min_size(pos2(0.0, 0.0), Vec2::new(1280.0, 720.0));
        let rect = overlay_rect(screen, [640, 360], [100, 50, 200, 20]);
        assert_eq!(rect.min, pos2(200.0, 100.0));
        assert_eq!(rect.size(), Vec2::new(400.0, 40.0));

        let screen = Rect::from_min_size(pos2(0.0, 0.0), Vec2::new(960.0, 540.0));
        let rect = overlay_rect(screen, [640, 360], [100, 50, 200, 20]);
        assert_eq!(rect.min, pos2(150.0, 75.0));
        assert_eq!(rect.size(), Vec2::new(300.0, 30.0));
    }

     
     
    #[test]
    fn button_rectangles_are_offset_by_the_screen_origin() {
        let screen = Rect::from_min_size(pos2(10.0, 20.0), Vec2::new(640.0, 360.0));
        let rect = overlay_rect(screen, [640, 360], [5, 6, 7, 8]);
        assert_eq!(rect.min, pos2(15.0, 26.0));
        assert_eq!(rect.size(), Vec2::new(7.0, 8.0));
    }

    #[test]
    fn vanilla_labels_pick_their_action_and_icon() {
        assert_eq!(button_kind("Back to Game"), ButtonKind::Resume);
        assert_eq!(button_kind("Resume"), ButtonKind::Resume);
        assert_eq!(button_kind("Options..."), ButtonKind::Options);
        assert_eq!(button_kind("Advancements"), ButtonKind::Advancements);
        assert_eq!(button_kind("Statistics"), ButtonKind::Statistics);
        assert_eq!(button_kind("Open to LAN"), ButtonKind::Lan);
        assert_eq!(button_kind("Save and Quit to Title"), ButtonKind::Quit);
        assert_eq!(button_kind("Disconnect"), ButtonKind::Quit);
        assert_eq!(button_kind("Something else"), ButtonKind::Other);
    }

     
     
    #[test]
    fn the_card_wraps_the_vanilla_button_block() {
        let viewport = Rect::from_min_size(pos2(0.0, 0.0), Vec2::new(1280.0, 720.0));
        let rects = vec![
            Rect::from_min_size(pos2(540.0, 300.0), Vec2::new(200.0, 20.0)),
            Rect::from_min_size(pos2(540.0, 330.0), Vec2::new(200.0, 20.0)),
        ];
        let layout = pause_layout(viewport, [640, 360], &rects).expect("a layout");
        for (button, vanilla) in layout.buttons.iter().zip(&rects) {
            assert_eq!(
                button.center(),
                vanilla.center(),
                "buttons stay centred on their vanilla slots"
            );
            assert!(
                button.width() >= vanilla.width(),
                "a single-column row should be at least vanilla width"
            );
            assert_eq!(button.height(), vanilla.height());
        }
        for rect in &layout.buttons {
            assert!(
                layout.card.contains_rect(*rect),
                "the card must contain {rect:?}"
            );
        }
        assert!(
            layout.card.top() <= layout.buttons[0].min.y - 40.0,
            "header space"
        );
        assert!(
            layout.card.bottom() >= layout.buttons[1].max.y + 20.0,
            "footer space"
        );
        assert!(layout.card.width() >= 288.0);

         
         
        let tiny = Rect::from_min_size(pos2(0.0, 0.0), Vec2::new(200.0, 200.0));
        let layout = pause_layout(tiny, [640, 360], &rects).expect("a layout");
        assert_eq!(layout.card.center().x, tiny.center().x);
        assert!(layout.card.top() >= 14.0 && layout.card.bottom() <= tiny.bottom() - 14.0);

        assert!(pause_layout(viewport, [0, 360], &rects).is_none());
        assert!(pause_layout(viewport, [640, 360], &[]).is_none());
    }

     
     
    fn painted(labels: &[&str]) -> (Vec<String>, Vec<egui::Shape>) {
        let ctx = Context::default();
        let viewport = Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 720.0));
        let size = [640, 360];
        let views: Vec<[i32; 4]> = labels
            .iter()
            .enumerate()
            .map(|(index, _)| [220, 120 + index as i32 * 26, 200, 20])
            .collect();
        let rects: Vec<Rect> = views
            .iter()
            .map(|rect| overlay_rect(viewport, size, *rect))
            .collect();
        let layout = pause_layout(viewport, size, &rects).expect("a layout");
        let mut texts = Vec::new();
        let mut shapes = Vec::new();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(viewport),
                ..Default::default()
            },
            |ctx| {
                let painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("pause")));
                let look = PauseLook {
                    style: UiStyle::default(),
                    pause: PauseStyle::default(),
                    reveal: 1.0,
                    time: 0.0,
                };
                paint_chrome(&painter, viewport, &layout, &look);
                for (index, (label, rect)) in labels.iter().zip(layout.buttons.iter()).enumerate() {
                    paint_button(
                        &painter,
                        &ButtonArt {
                            rect: *rect,
                            label,
                            kind: button_kind(label),
                            hover: if index == 0 { 1.0 } else { 0.0 },
                            press: 0.0,
                            reveal: 1.0,
                        },
                        &look,
                    );
                }
                paint_footer(&painter, &layout, &look);
            },
        );
        for clipped in output.shapes {
            if let egui::Shape::Text(text) = &clipped.shape {
                texts.push(text.galley.text().to_owned());
            }
            shapes.push(clipped.shape);
        }
        (texts, shapes)
    }

    #[test]
    fn the_replacement_paints_its_brand_header_buttons_and_hints() {
        let labels = ["Back to Game", "Options...", "Save and Quit to Title"];
        let (texts, shapes) = painted(&labels);
        assert!(texts.iter().any(|text| text == "GAME PAUSED"), "{texts:?}");
        assert!(texts.iter().any(|text| text == "INFERED"), "{texts:?}");
        for label in labels {
            assert!(texts.iter().any(|text| text == label), "missing {label}");
        }
        assert!(
            texts.iter().any(|text| text.contains("client menu")),
            "missing the key hints: {texts:?}"
        );
         
        let rects = shapes
            .iter()
            .filter(|shape| matches!(shape, egui::Shape::Rect(_)))
            .count();
        assert!(rects >= 10, "only {rects} rectangles painted");
        assert!(
            shapes
                .iter()
                .any(|shape| matches!(shape, egui::Shape::Mesh(_))),
            "the backdrop wash is missing"
        );
    }

     
     
    #[test]
    fn the_primary_row_answers_hover_and_press() {
        let rect = Rect::from_min_size(pos2(100.0, 100.0), Vec2::new(300.0, 40.0));
        let rest = button_shapes(rect, 0.0, 0.0);
        let hovered = button_shapes(rect, 1.0, 0.0);
        let pressed = button_shapes(rect, 0.0, 1.0);

        let main = |shapes: &[egui::Shape]| {
            shapes
                .iter()
                .find_map(|shape| match shape {
                    egui::Shape::Rect(rect)
                        if rect.stroke.width > 0.0 && rect.stroke.color != Color32::TRANSPARENT =>
                    {
                        Some(*rect)
                    }
                    _ => None,
                })
                .expect("the button body")
        };
        let rest_main = main(&rest);
        let hover_main = main(&hovered);
        let press_main = main(&pressed);
        assert!(
            hover_main.fill.r() > rest_main.fill.r() && hover_main.fill.g() >= rest_main.fill.g(),
            "hover must warm the primary fill: {:?} -> {:?}",
            rest_main.fill,
            hover_main.fill
        );
        assert!(
            press_main.rect.size().x < rest_main.rect.size().x,
            "pressing must squeeze the row"
        );
        assert!(
            hovered.iter().any(|shape| matches!(
                shape,
                egui::Shape::Rect(glow) if glow.fill.a() > 0 && glow.fill.a() < 40
            )),
            "a hovered row must draw its bloom"
        );
    }

    fn button_shapes(rect: Rect, hover: f32, press: f32) -> Vec<egui::Shape> {
        let ctx = Context::default();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
                ..Default::default()
            },
            |ctx| {
                let painter =
                    ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("button_test")));
                let look = PauseLook {
                    style: UiStyle::default(),
                    pause: PauseStyle::default(),
                    reveal: 1.0,
                    time: 0.0,
                };
                paint_button(
                    &painter,
                    &ButtonArt {
                        rect,
                        label: "Back to Game",
                        kind: ButtonKind::Resume,
                        hover,
                        press,
                        reveal: 1.0,
                    },
                    &look,
                );
            },
        );
        output
            .shapes
            .into_iter()
            .map(|clipped| clipped.shape)
            .collect()
    }

     
     
    #[test]
    fn every_action_has_an_icon() {
        for kind in [
            ButtonKind::Resume,
            ButtonKind::Options,
            ButtonKind::Advancements,
            ButtonKind::Statistics,
            ButtonKind::Lan,
            ButtonKind::Quit,
            ButtonKind::Other,
        ] {
            let ctx = Context::default();
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(200.0, 100.0))),
                    ..Default::default()
                },
                |ctx| {
                    let painter = ctx.layer_painter(LayerId::new(
                        Order::Foreground,
                        Id::new("icon_test").with(format!("{kind:?}")),
                    ));
                    paint_icon(&painter, kind, Pos2::new(100.0, 50.0), 8.0, Color32::WHITE);
                },
            );
            assert!(!output.shapes.is_empty(), "{kind:?} painted no glyph",);
        }
    }

     
     
     
    #[test]
    fn the_header_keeps_the_brand_and_title_apart() {
        for (viewport, gui) in [
            (
                Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 560.0)),
                [640, 360],
            ),
            (
                Rect::from_min_size(Pos2::ZERO, Vec2::new(1600.0, 1000.0)),
                [640, 360],
            ),
            (
                Rect::from_min_size(Pos2::ZERO, Vec2::new(600.0, 420.0)),
                [640, 360],
            ),
        ] {
            let rects: Vec<Rect> = (0..5)
                .map(|index| overlay_rect(viewport, gui, [220, 96 + index * 26, 200, 20]))
                .collect();
            let layout = pause_layout(viewport, gui, &rects).expect("a layout");
            let ctx = Context::default();
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(viewport),
                    ..Default::default()
                },
                |ctx| {
                    let painter =
                        ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("header_test")));
                    let look = PauseLook {
                        style: UiStyle::default(),
                        pause: PauseStyle::default(),
                        reveal: 1.0,
                        time: 0.0,
                    };
                    paint_chrome(&painter, viewport, &layout, &look);
                },
            );
            let text = |label: &str| {
                output
                    .shapes
                    .iter()
                    .find_map(|clipped| match &clipped.shape {
                        egui::Shape::Text(text) if text.galley.text() == label => {
                            Some(Rect::from_min_size(text.pos, text.galley.size()))
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("missing {label}"))
            };
            let brand = text("INFERED");
            let title = text("GAME PAUSED");
            assert!(
                brand.bottom() <= title.top(),
                "the brand {brand:?} overlaps the title {title:?} at {viewport:?}"
            );
            assert!(
                title.bottom() <= layout.buttons[0].top(),
                "the title {title:?} runs into the first button at {viewport:?}"
            );
            assert!(
                layout.card.contains_rect(title),
                "the title {title:?} leaves the card {:?} at {viewport:?}",
                layout.card
            );
        }
    }

     
     
     
    #[test]
    fn the_pause_menu_preview_renders_lit_pixels() {
        let viewport = Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 560.0));
        let labels = [
            "Back to Game",
            "Advancements",
            "Statistics",
            "Options...",
            "Save and Quit to Title",
        ];
        let ctx = Context::default();
        let mut textures = HashMap::new();
        let mut primitives = Vec::new();
        for frame in 0..4 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(viewport),
                    time: Some(frame as f64 / 60.0),
                    predicted_dt: 1.0 / 60.0,
                    ..Default::default()
                },
                |ctx| {
                    pause_menu_preview(ctx, viewport, &labels);
                },
            );
            gather_textures(&mut textures, &output.textures_delta);
            primitives = ctx.tessellate(output.shapes, 1.0);
        }
        let image = rasterize(
            primitives,
            &textures,
            viewport.width() as usize,
            viewport.height() as usize,
            [0.055, 0.055, 0.07, 1.0],
        );
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../target/pause-menu-preview.png");
        image.save(&path).expect("the pause menu preview png saves");
        let lit = image
            .pixels()
            .filter(|pixel| {
                pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32 > 90 && pixel[3] > 128
            })
            .count();
        println!("pause menu preview: {lit} lit pixels -> {path:?}");
        assert!(lit > 6_000, "the pause menu is barely painted ({lit})");
    }

     
    fn pause_menu_preview(ctx: &Context, viewport: Rect, labels: &[&str]) {
        let size = [640, 360];
        let rects: Vec<Rect> = labels
            .iter()
            .enumerate()
            .map(|(index, _)| overlay_rect(viewport, size, [220, 96 + index as i32 * 26, 200, 20]))
            .collect();
        let layout = pause_layout(viewport, size, &rects).expect("a layout");
        let painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("pause_preview")));
        let look = PauseLook {
            style: UiStyle::default(),
            pause: PauseStyle::default(),
            reveal: 1.0,
            time: 0.35,
        };
        paint_chrome(&painter, viewport, &layout, &look);
        for (index, (label, rect)) in labels.iter().zip(layout.buttons.iter()).enumerate() {
            paint_button(
                &painter,
                &ButtonArt {
                    rect: *rect,
                    label,
                    kind: button_kind(label),
                    hover: if index == 1 { 1.0 } else { 0.0 },
                    press: 0.0,
                    reveal: 1.0,
                },
                &look,
            );
        }
        paint_footer(&painter, &layout, &look);
    }
}
