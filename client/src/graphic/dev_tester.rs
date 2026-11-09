 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::config;
use crate::graphic::notification::{Notification, NotificationType};
use crate::graphic::theme;
use crate::module::{ModuleCategory, ModuleId};
use egui::{Align2, Context, Frame, Id, Layout, Margin, Order, RichText, Rounding, Stroke, Vec2};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};

 
const KEY_ESCAPE: i32 = 256;
const KEY_ENTER: i32 = 257;
const KEY_Y: i32 = 89;
const KEY_PAGE_DOWN: i32 = 267;
 
const MOD_CONTROL: i32 = 0x2;

 
const WIDTH: f32 = 340.0;
const EXPANDED_WIDTH: f32 = 470.0;
 
const EDGE: f32 = 18.0;
 
const RESULT_TOAST: f32 = 1.4;

 
const OVERVIEW_FILE: &str = "dev_overview.md";

 
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
     
    Idle,
     
    Testing,
     
    Explaining,
}

 
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
     
    pub alphabetical: bool,
     
    pub auto_enable: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            alphabetical: false,
            auto_enable: true,
        }
    }
}

 
 
 
#[derive(Clone, Debug)]
struct Outcome {
    id: ModuleId,
    ok: bool,
    reason: String,
    category: String,
    keybind: String,
}

 
struct Round {
    phase: Phase,
    prepared: bool,
    queue: Vec<ModuleId>,
    index: usize,
    outcomes: Vec<Outcome>,
    note: String,
    alphabetical: bool,
    auto_enable: bool,
}

impl Round {
    fn idle() -> Self {
        Self {
            phase: Phase::Idle,
            prepared: false,
            queue: Vec::new(),
            index: 0,
            outcomes: Vec::new(),
            note: String::new(),
            alphabetical: false,
            auto_enable: true,
        }
    }

    fn current(&self) -> Option<ModuleId> {
        self.queue.get(self.index).copied()
    }
}

 
 
static ACTIVE: AtomicBool = AtomicBool::new(false);
 
static CURSOR_SYNCED: AtomicBool = AtomicBool::new(false);
 
 
static FOCUS_NOTE: AtomicBool = AtomicBool::new(false);
 
static PENDING: LazyLock<Mutex<Vec<(ModuleId, bool)>>> = LazyLock::new(|| Mutex::new(Vec::new()));
 
static NEEDS_SAVE: AtomicBool = AtomicBool::new(false);
 
static ROUND: LazyLock<Mutex<Round>> = LazyLock::new(|| Mutex::new(Round::idle()));

 
 
 
fn round() -> std::sync::MutexGuard<'static, Round> {
    ROUND
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

 
pub fn is_active() -> bool {
    ACTIVE.load(Ordering::SeqCst)
}

 
 
 
pub fn is_typing() -> bool {
    is_active() && round().phase == Phase::Explaining
}

 
pub fn overview_path() -> PathBuf {
    config::base_dir().join(OVERVIEW_FILE)
}

 

 
 
 
 
 
 
pub fn start(options: Options) {
    if ACTIVE.swap(true, Ordering::SeqCst) {
        return;  
    }
    {
        let mut state = round();
        *state = Round {
            phase: Phase::Testing,
            prepared: false,
            queue: Vec::new(),
            index: 0,
            outcomes: Vec::new(),
            note: String::new(),
            alphabetical: options.alphabetical,
            auto_enable: options.auto_enable,
        };
    }
    Notification::send(
        NotificationType::Info,
        "Module Tester",
        "Preparing the review list…",
    );
    std::thread::spawn(prepare_round);
}

 
 
 
 
pub fn pump() {
    let actions = {
        let mut pending = PENDING
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if pending.is_empty() {
            Vec::new()
        } else {
            std::mem::take(&mut *pending)
        }
    };
    let dirty = NEEDS_SAVE.swap(false, Ordering::SeqCst);
    if actions.is_empty() && !dirty {
        return;
    }
    for (id, enabled) in actions {
         
         
         
        if enabled {
            let state = round();
            if state.phase == Phase::Idle || state.current() != Some(id) {
                continue;
            }
        }
        set_module_enabled(id, enabled);
    }
    crate::config::save();
}

 
 
fn queue_action(id: ModuleId, enabled: bool) {
    let mut pending = PENDING
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    pending.push((id, enabled));
}

 
 
pub fn stop() {
    if !ACTIVE.swap(false, Ordering::SeqCst) {
        return;
    }
     
     
     
    let current = {
        let mut state = round();
        let current = state.current();
        state.phase = Phase::Idle;
        state.note.clear();
        write_overview(&state);
        current
    };
    if let Some(id) = current {
        queue_action(id, false);
    }
}

 
 
fn prepare_round() {
    let mut entries: Vec<(ModuleCategory, ModuleId)> = Vec::new();
    for handle in crate::state::client().modules.handles() {
        let Ok(mut module) = handle.lock() else {
            continue;
        };
        let id = module.get_module_data().id;
        let category = module.get_module_data().category;
         
         
        if id != ModuleId::ModuleTester {
            let was_enabled = module.get_module_data().enabled;
            module.get_module_data_mut().enabled = false;
            if was_enabled && crate::state::client().modules.is_active() {
                if let Err(error) = module.on_stop() {
                    log::warn!(
                        "Module Tester: could not stop {} before the round: {error}",
                        id.display_name()
                    );
                }
            }
        }
        entries.push((category, id));
    }

    if !ACTIVE.load(Ordering::SeqCst) {
        return;  
    }

    let (queue_len, auto_enable, path) = {
        let mut state = round();
        let queue = plan(entries, state.alphabetical);
        state.queue = queue;
        state.prepared = true;
        state.phase = Phase::Testing;
        write_overview(&state);
        (state.queue.len(), state.auto_enable, overview_path())
    };

    Notification::send_with_time(
        NotificationType::Info,
        "Module Tester",
        &format!("{queue_len} modules queued — Y = works, Page Down = broken"),
        6.0,
    );

     
     
    log::info!("Module Tester: overview at {}", path.display());

     
     
    NEEDS_SAVE.store(true, Ordering::SeqCst);
    if auto_enable {
        if let Some(id) = round().current() {
            queue_action(id, true);
        }
    }
}

 
fn settle(ok: bool, reason: String) {
    let (id, auto_enable) = {
        let state = round();
        match state.current() {
            Some(id) => (id, state.auto_enable),
            None => return,
        }
    };
    let (category, keybind, _) = module_facts(id);

     
    set_module_enabled(id, false);

    let next = {
        let mut state = round();
        state.outcomes.push(Outcome {
            id,
            ok,
            reason: reason.clone(),
            category,
            keybind,
        });
        state.note.clear();
        state.index += 1;
        state.phase = Phase::Testing;
        write_overview(&state);
        state.current()
    };

    if ok {
        Notification::send_with_time(
            NotificationType::Info,
            "Module Tester",
            &format!("{} works", id.display_name()),
            RESULT_TOAST,
        );
    } else {
        Notification::send_with_time(
            NotificationType::Warning,
            "Module Tester",
            &format!("{} logged as broken", id.display_name()),
            RESULT_TOAST * 2.0,
        );
    }

    match next {
        Some(id) => {
            if auto_enable {
                set_module_enabled(id, true);
            }
        }
        None => finish(),
    }
}

 
 
fn finish() {
    ACTIVE.store(false, Ordering::SeqCst);
    let (tested, broken, path) = {
        let mut state = round();
        state.phase = Phase::Idle;
        state.note.clear();
        let broken = state.outcomes.iter().filter(|outcome| !outcome.ok).count();
        write_overview(&state);
        (state.outcomes.len(), broken, overview_path())
    };

    Notification::send_with_time(
        if broken == 0 {
            NotificationType::Info
        } else {
            NotificationType::Warning
        },
        "Module Tester",
        &format!("{tested} tested, {broken} broken — see {}", path.display()),
        9.0,
    );
     
     
    crate::graphic::input::request_module_toggle(ModuleId::ModuleTester);
}

 
 
 
fn set_module_enabled(id: ModuleId, enabled: bool) {
    let Some(handle) = crate::state::client().modules.get(id) else {
        return;
    };
    let Ok(mut module) = handle.lock() else {
        return;
    };
    if module.get_module_data().enabled == enabled {
        return;
    }
    module.get_module_data_mut().set_enabled(enabled);
    if crate::state::client().modules.is_active() {
        let result = if enabled {
            module.on_start()
        } else {
            module.on_stop()
        };
        if let Err(error) = result {
            log::warn!(
                "Module Tester: {} failed to {}: {error}",
                id.display_name(),
                if enabled { "start" } else { "stop" }
            );
        }
    }
}

 

 
 
pub fn handle_key(key: i32, mods: i32) -> bool {
    if !is_active() {
        return false;
    }
     
     
    let phase = round().phase;
    match phase {
        Phase::Testing => match key {
            KEY_Y => {
                settle(true, String::new());
                true
            }
            KEY_PAGE_DOWN => {
                let mut state = round();
                if state.current().is_some() {
                    state.phase = Phase::Explaining;
                    state.note.clear();
                    FOCUS_NOTE.store(true, Ordering::Relaxed);
                }
                true
            }
             
             
            KEY_ESCAPE => false,
            _ => false,
        },
        Phase::Explaining => {
            if key == KEY_ESCAPE {
                let mut state = round();
                state.phase = Phase::Testing;
                state.note.clear();
                true
            } else if key == KEY_ENTER && mods & MOD_CONTROL != 0 {
                submit_note();
                true
            } else {
                false
            }
        }
        Phase::Idle => false,
    }
}

 
pub fn submit_note() {
    let reason = {
        let state = round();
        if state.phase != Phase::Explaining {
            return;
        }
        let typed = state.note.trim().to_string();
        if typed.is_empty() {
            "no reason given".to_string()
        } else {
            typed
        }
    };
    settle(false, reason);
}

 

 
 
 
 
pub fn sync_cursor() {
    let open = crate::graphic::dev_popup::is_open();
    if CURSOR_SYNCED.swap(open, Ordering::Relaxed) != open {
        crate::graphic::input::set_dev_cursor(open);
    }
}

 

 
fn write_overview(state: &Round) {
    if let Err(error) = std::fs::write(overview_path(), render_overview(state)) {
        log::warn!("Module Tester: could not write {OVERVIEW_FILE}: {error}");
    }
}

 
 
fn render_overview(state: &Round) -> String {
    let tested = state.outcomes.len();
    let broken = state.outcomes.iter().filter(|outcome| !outcome.ok).count();
    let pending: Vec<ModuleId> = state.queue.iter().skip(state.index).copied().collect();

    let mut text = String::new();
    text.push_str("# Infered dev overview\n\n");
    text.push_str(&format!(
        "Written by the Module Tester in a `devtest` build at {}.\n\n",
        utc_stamp()
    ));
    text.push_str(&format!(
        "- Modules in the round: {}\n- Tested: {}\n- Working: {}\n- Broken: {}\n",
        state.queue.len(),
        tested,
        tested - broken,
        broken
    ));
    if state.phase == Phase::Idle && !pending.is_empty() {
        text.push_str(&format!("- Not reached: {}\n", pending.len()));
    }
    text.push('\n');

    text.push_str("## Broken modules\n\n");
    let mut any_broken = false;
    for outcome in state.outcomes.iter().filter(|outcome| !outcome.ok) {
        any_broken = true;
        text.push_str(&format!("### {}\n\n", outcome.id.display_name()));
        text.push_str(&format!("- Category: {}\n", outcome.category));
        text.push_str(&format!("- Keybind: {}\n", outcome.keybind));
        text.push_str(&format!("- Reason: {}\n\n", outcome.reason));
    }
    if !any_broken {
        text.push_str("_None — every tested module worked._\n\n");
    }

    if !pending.is_empty() {
        text.push_str("## Not reached\n\n");
        for id in &pending {
            text.push_str(&format!("- {}\n", id.display_name()));
        }
        text.push('\n');
    }

    text.push_str("## Working modules\n\n");
    let mut any_working = false;
    for outcome in state.outcomes.iter().filter(|outcome| outcome.ok) {
        any_working = true;
        text.push_str(&format!("- {}\n", outcome.id.display_name()));
    }
    if !any_working {
        text.push_str("_None yet._\n");
    }

    text
}

 
 
 
fn module_facts(id: ModuleId) -> (String, String, bool) {
    let Some(handle) = crate::state::client().modules.get(id) else {
        return ("unknown".to_string(), "unknown".to_string(), false);
    };
    let Ok(module) = handle.lock() else {
        return ("unknown".to_string(), "unknown".to_string(), false);
    };
    let data = module.get_module_data();
    (
        data.category.display_name().to_string(),
        data.key_bind.to_string(),
        data.enabled,
    )
}

 
 
fn utc_stamp() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    let (days, rest) = (seconds / 86_400, seconds % 86_400);
    let (year, month, day) = civil_from_days(days as i64);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02} UTC",
        rest / 3600,
        (rest % 3600) / 60,
        rest % 60
    )
}

 
 
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

 

 
pub fn draw(ctx: &Context) {
    if !is_active() {
        return;
    }
     
    ctx.request_repaint();

    let (phase, index, total, prepared, working, broken) = {
        let state = round();
        (
            state.phase,
            state.index,
            state.queue.len(),
            state.prepared,
            state.outcomes.iter().filter(|outcome| outcome.ok).count(),
            state.outcomes.iter().filter(|outcome| !outcome.ok).count(),
        )
    };
    if phase == Phase::Idle {
        return;
    }
    let current = round().current();
     
     
    let facts = current.map(module_facts);

    let explaining = phase == Phase::Explaining;
    let width = if explaining { EXPANDED_WIDTH } else { WIDTH };
    let mut note = {
        let state = round();
        state.note.clone()
    };

    egui::Area::new(Id::new("dev_module_tester"))
        .order(Order::Foreground)
        .anchor(Align2::RIGHT_TOP, Vec2::new(-EDGE, EDGE))
        .show(ctx, |ui| {
            let window = Frame::none()
                .fill(theme::SURFACE)
                .stroke(Stroke::new(1.0_f32, theme::BORDER))
                .rounding(Rounding::same(theme::RADIUS))
                .shadow(ui.style().visuals.window_shadow)
                .inner_margin(Margin::same(16.0))
                .show(ui, |ui| {
                    ui.set_width(width);
                    ui.spacing_mut().item_spacing.y = 8.0;

                     
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Module Tester")
                                .size(16.0)
                                .strong()
                                .color(theme::TEXT),
                        );
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            let counter = if total == 0 {
                                "preparing".to_string()
                            } else {
                                format!("{} / {}", (index + 1).min(total), total)
                            };
                            ui.label(RichText::new(counter).size(11.5).color(theme::TEXT_MUTED));
                        });
                    });

                     
                    let answered = working + broken;
                    let fraction = if total == 0 {
                        0.0
                    } else {
                        answered as f32 / total as f32
                    };
                    ui.add(
                        egui::ProgressBar::new(fraction)
                            .desired_width(width)
                            .fill(theme::accent()),
                    );

                    if !prepared {
                        ui.label(
                            RichText::new("Disabling every module before the first one comes up…")
                                .size(11.5)
                                .color(theme::TEXT_MUTED),
                        );
                        return;
                    }

                    let Some(id) = current else {
                        ui.label(
                            RichText::new("The round is over.")
                                .size(12.0)
                                .color(theme::TEXT_DIM),
                        );
                        return;
                    };

                     
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(id.display_name())
                            .size(22.0)
                            .strong()
                            .color(theme::TEXT),
                    );
                    if let Some((category, keybind, enabled)) = &facts {
                        let state = if *enabled { "live" } else { "armed off" };
                        ui.label(
                            RichText::new(format!("{category} · keybind {keybind} · {state}"))
                                .size(11.5)
                                .color(theme::TEXT_MUTED),
                        );
                    }
                    ui.add_space(2.0);

                    if !explaining {
                        ui.label(
                            RichText::new("Test it in the world, then answer:")
                                .size(11.5)
                                .color(theme::TEXT_DIM),
                        );
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Y — works")
                                    .size(13.0)
                                    .strong()
                                    .color(theme::SUCCESS),
                            );
                            ui.add_space(16.0);
                            ui.label(
                                RichText::new("Page Down — doesn't work")
                                    .size(13.0)
                                    .strong()
                                    .color(theme::DANGER),
                            );
                        });
                        ui.label(
                            RichText::new(format!("{working} working · {broken} broken"))
                                .size(11.0)
                                .color(theme::TEXT_MUTED),
                        );
                    } else {
                        ui.label(
                            RichText::new(format!(
                                "{} doesn't work — what is wrong?",
                                id.display_name()
                            ))
                            .size(12.0)
                            .color(theme::TEXT_DIM),
                        );
                        let field = ui.add(
                            egui::TextEdit::multiline(&mut note)
                                .desired_rows(3)
                                .desired_width(f32::INFINITY)
                                .hint_text("e.g. clicking the module toggles it twice"),
                        );
                         
                         
                        if FOCUS_NOTE.swap(false, Ordering::Relaxed) {
                            field.request_focus();
                        }
                        ui.label(
                            RichText::new("Ctrl+Enter logs it · Esc goes back")
                                .size(10.5)
                                .color(theme::TEXT_MUTED),
                        );
                    }

                    ui.add_space(2.0);
                     
                     
                    ui.label(
                        RichText::new("Broken ones land in dev_overview.md")
                            .size(10.5)
                            .color(theme::TEXT_MUTED),
                    );
                    ui.label(
                        RichText::new("Esc opens the game menu; the round stays up")
                            .size(10.5)
                            .color(theme::TEXT_MUTED),
                    );
                    ui.label(
                        RichText::new("Turn Module Tester off to stop the round")
                            .size(10.5)
                            .color(theme::TEXT_MUTED),
                    );
                });
            let _ = window;
        });

     
    if explaining {
        let mut state = round();
        if state.phase == Phase::Explaining && state.note != note {
            state.note = note;
        }
    }
}

 

 
 
 
fn skipped(id: ModuleId) -> bool {
    matches!(
        id,
        ModuleId::ModuleTester | ModuleId::DevMode | ModuleId::Panic
    )
}

 
 
fn category_rank(category: ModuleCategory) -> u8 {
    match category {
        ModuleCategory::Combat => 0,
        ModuleCategory::Movement => 1,
        ModuleCategory::Render => 2,
        ModuleCategory::Visuals => 3,
        ModuleCategory::Player => 4,
        ModuleCategory::World => 5,
        ModuleCategory::Performance => 6,
        ModuleCategory::Misc => 7,
        ModuleCategory::Macros => 8,
        ModuleCategory::Dev => 9,
         
         
        ModuleCategory::Search => 10,
    }
}

 
fn plan(mut entries: Vec<(ModuleCategory, ModuleId)>, alphabetical: bool) -> Vec<ModuleId> {
    entries.retain(|(_, id)| !skipped(*id));
    if alphabetical {
        entries.sort_by_key(|(_, id)| id.display_name());
    } else {
        entries.sort_by_key(|(category, id)| (category_rank(*category), id.display_name()));
    }
    entries.into_iter().map(|(_, id)| id).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

     
     
    #[test]
    fn the_queue_walks_the_menu_order() {
        let entries = vec![
            (ModuleCategory::Misc, ModuleId::Panic),
            (ModuleCategory::Combat, ModuleId::KillAura),
            (ModuleCategory::Dev, ModuleId::ModuleTester),
            (ModuleCategory::Combat, ModuleId::Aimbot),
        ];
        assert_eq!(
            plan(entries.clone(), false),
            vec![ModuleId::Aimbot, ModuleId::KillAura],
            "the tester itself and Panic are never queued, and Combat comes first"
        );
        assert_eq!(
            plan(entries, true),
            vec![ModuleId::Aimbot, ModuleId::KillAura],
            "alphabetical order drops the category grouping, not the exclusions"
        );
    }

     
    #[test]
    fn categories_keep_their_menu_order() {
        let ranks = [
            category_rank(ModuleCategory::Combat),
            category_rank(ModuleCategory::Movement),
            category_rank(ModuleCategory::Render),
            category_rank(ModuleCategory::Visuals),
            category_rank(ModuleCategory::Player),
            category_rank(ModuleCategory::World),
            category_rank(ModuleCategory::Performance),
            category_rank(ModuleCategory::Misc),
            category_rank(ModuleCategory::Macros),
            category_rank(ModuleCategory::Dev),
        ];
        let mut sorted = ranks;
        sorted.sort_unstable();
        assert_eq!(ranks, sorted, "the ranks must already be in menu order");
    }

     
    #[test]
    fn epoch_days_convert_to_calendar_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_723), (2024, 1, 1));
         
        assert_eq!(civil_from_days(19_782), (2024, 2, 29));
    }

     
    #[test]
    fn an_empty_round_still_writes_an_overview() {
        let mut state = Round::idle();
        state.phase = Phase::Testing;
        state.outcomes.push(Outcome {
            id: ModuleId::KillAura,
            ok: false,
            reason: "no hits".to_string(),
            category: "Combat".to_string(),
            keybind: "None".to_string(),
        });
        let text = render_overview(&state);
        assert!(text.contains("# Infered dev overview"));
        assert!(text.contains("### KillAura"));
        assert!(text.contains("- Reason: no hits"));
        assert!(text.contains("_None yet._"));
    }
}
