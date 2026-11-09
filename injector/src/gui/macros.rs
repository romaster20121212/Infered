 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use egui::{
    pos2, vec2, Align, Align2, Color32, CursorIcon, FontId, Id, Layout, Margin, Order, Pos2, Rect,
    RichText, Rounding, ScrollArea, Sense, Shape, Stroke, TextEdit, TextureHandle, Ui, UiBuilder,
    Vec2,
};
use infered_macros::{
    action_summary, delete, duplicate, export, import, load_library, macros_dir, new_template,
    save, save_text, trigger_summary, validate, Macro,
};
use serde_json::Value as Json;

use super::config::toggle_switch;
use super::theme::{lerp_color, pulse, with_alpha, Palette};
use super::widgets::{
    count_up, ease_out_back, elided, glossy_pill, paint_toggle, radial_glow, sweep,
    top_light_rounded, tracked, PillIcon, PillStyle,
};
use super::{ease_out_cubic, paint_notice, InjectorGui};

 
const LIBRARY_W: f32 = 244.0;
const GAP: f32 = 14.0;
 
const HEADER_H: f32 = 44.0;
 
const CARD_H: f32 = 46.0;
 
const NEW_PILL: Vec2 = Vec2::new(116.0, 28.0);

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortBy {
    Name,
    Modified,
    Triggers,
    Enabled,
}

impl SortBy {
    const ALL: [SortBy; 4] = [
        SortBy::Name,
        SortBy::Modified,
        SortBy::Triggers,
        SortBy::Enabled,
    ];

    fn label(self) -> &'static str {
        match self {
            SortBy::Name => "Name",
            SortBy::Modified => "Modified",
            SortBy::Triggers => "Triggers",
            SortBy::Enabled => "Enabled",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Tab {
    #[default]
    Visual,
    Json,
}

 
#[derive(Clone)]
struct Entry {
    file_name: String,
    id: String,
    name: String,
    description: String,
    triggers: Vec<String>,
    enabled: bool,
    modified: u64,
    action_count: usize,
    error: Option<String>,
    warning: Option<String>,
}

 
 
#[derive(Default)]
struct Editor {
    file_name: String,
    doc: Json,
    raw: String,
    saved: String,
    tab: Tab,
    buffers: HashMap<String, String>,
    undo: Vec<Json>,
    redo: Vec<Json>,
    issues: Vec<String>,
    raw_error: Option<String>,
}

impl Editor {
    fn dirty(&self) -> bool {
        self.raw != self.saved
    }

    fn buffer(&mut self, key: &str, initial: String) -> String {
        self.buffers
            .entry(key.to_string())
            .or_insert(initial)
            .clone()
    }

    fn put_buffer(&mut self, key: &str, value: String) {
        self.buffers.insert(key.to_string(), value);
    }

    fn resync_raw(&mut self) {
        self.raw = serde_json::to_string_pretty(&self.doc).unwrap_or_default();
    }

    fn snapshot(&mut self, before: Json) {
        self.undo.push(before);
        if self.undo.len() > 64 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn undo(&mut self) -> bool {
        let Some(previous) = self.undo.pop() else {
            return false;
        };
        self.redo.push(std::mem::replace(&mut self.doc, previous));
        self.buffers.clear();
        self.resync_raw();
        true
    }

    fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push(std::mem::replace(&mut self.doc, next));
        self.buffers.clear();
        self.resync_raw();
        true
    }

    fn validate(&mut self) {
        self.issues.clear();
        match serde_json::from_value::<Macro>(self.doc.clone()) {
            Ok(macro_) => {
                for issue in validate(&macro_) {
                    self.issues
                        .push(format!("{}: {}", issue.path, issue.message));
                }
            }
            Err(error) => self.issues.push(format!("document: {error}")),
        }
    }

    fn apply_raw(&mut self) -> bool {
        match infered_macros::parse_text(&self.raw) {
            Ok(macro_) => match serde_json::to_value(&macro_) {
                Ok(value) => {
                    self.doc = value;
                    self.raw_error = None;
                    self.buffers.clear();
                    self.validate();
                    self.resync_raw();
                    true
                }
                Err(error) => {
                    self.raw_error = Some(error.to_string());
                    false
                }
            },
            Err(error) => {
                self.raw_error = Some(error);
                false
            }
        }
    }
}

 
pub struct MacrosPage {
    opened_at: f64,
    entries: Vec<Entry>,
    selected: Option<String>,
    editor: Option<Editor>,
    search: String,
    sort: SortBy,
    sort_open: bool,
    notice: Option<(String, bool)>,
    confirm_delete: Option<String>,
    last_scan: f64,
    fingerprint: u64,
}

impl Default for MacrosPage {
    fn default() -> Self {
        Self {
            opened_at: -10.0,
            entries: Vec::new(),
            selected: None,
            editor: None,
            search: String::new(),
            sort: SortBy::Name,
            sort_open: false,
            notice: None,
            confirm_delete: None,
            last_scan: -10.0,
            fingerprint: 0,
        }
    }
}

impl MacrosPage {
     
     
    pub(super) fn open(&mut self, now: f64) {
        self.opened_at = now;
        self.confirm_delete = None;
        self.sort_open = false;
        self.reload(true);
        self.last_scan = now;
    }

    fn directory() -> PathBuf {
        let base = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        macros_dir(&base)
    }

     
     
    fn reload(&mut self, force: bool) {
        let dir = Self::directory();
        let _ = std::fs::create_dir_all(&dir);
        let library = load_library(&dir);
        let mut entries: Vec<Entry> = library
            .macros
            .iter()
            .map(|file| {
                let modified = file
                    .modified
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .map(|duration| duration.as_secs())
                    .unwrap_or(0);
                Entry {
                    file_name: file.file_name.clone(),
                    id: file.macro_.id.clone(),
                    name: file.macro_.name.clone(),
                    description: file.macro_.description.clone(),
                    triggers: file.macro_.triggers.iter().map(trigger_summary).collect(),
                    enabled: file.macro_.enabled,
                    modified,
                    action_count: file.macro_.actions.len(),
                    error: None,
                    warning: if file.macro_.actions.is_empty() {
                        Some("no actions".to_string())
                    } else {
                        None
                    },
                }
            })
            .collect();
        for broken in &library.broken {
            entries.push(Entry {
                file_name: broken.file_name.clone(),
                id: String::new(),
                name: broken.file_name.clone(),
                description: String::new(),
                triggers: Vec::new(),
                enabled: false,
                modified: 0,
                action_count: 0,
                error: Some(broken.error.clone()),
                warning: None,
            });
        }
        entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        let fingerprint = directory_fingerprint(&dir);
        if !force && fingerprint == self.fingerprint {
            return;
        }
        self.fingerprint = fingerprint;
        self.entries = entries;

        if self.selected.is_none() {
            self.selected = self.entries.first().map(|entry| entry.file_name.clone());
        }
        let still_there = self
            .selected
            .as_ref()
            .is_some_and(|file| self.entries.iter().any(|entry| &entry.file_name == file));
        if !still_there {
            self.selected = self.entries.first().map(|entry| entry.file_name.clone());
            self.editor = None;
        }
        let dirty = self.editor.as_ref().is_some_and(Editor::dirty);
        if !dirty && self.editor.is_some() {
            if let Some(file) = self.selected.clone() {
                self.load_editor(&file);
            }
        } else if self.editor.is_none() {
            if let Some(file) = self.selected.clone() {
                self.load_editor(&file);
            }
        }
    }

     
     
    fn poll(&mut self, now: f64) {
        if now - self.last_scan < 1.0 {
            return;
        }
        self.last_scan = now;
        let dir = Self::directory();
        let fingerprint = directory_fingerprint(&dir);
        if fingerprint != self.fingerprint {
            self.reload(false);
        }
    }

    fn load_editor(&mut self, file_name: &str) {
        let path = Self::directory().join(file_name);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => {
                self.notice = Some((format!("could not read {file_name}: {error}"), false));
                return;
            }
        };
        let parsed = infered_macros::parse_text(&text);
        let mut editor = Editor {
            file_name: file_name.to_string(),
            raw: text.clone(),
            saved: text,
            tab: Tab::Visual,
            ..Editor::default()
        };
        match parsed {
            Ok(macro_) => {
                editor.doc = serde_json::to_value(&macro_).unwrap_or(Json::Null);
                editor.validate();
            }
            Err(error) => {
                editor.raw_error = Some(error.clone());
                self.notice = Some((format!("{file_name}: {error}"), false));
            }
        }
        self.editor = Some(editor);
    }

    fn select(&mut self, file_name: &str) {
        if self.editor.as_ref().is_some_and(Editor::dirty) {
            self.notice = Some((
                "discarded unsaved edits — save first to keep them".to_string(),
                false,
            ));
        }
        self.selected = Some(file_name.to_string());
        self.load_editor(file_name);
    }

    fn create(&mut self) {
        let dir = Self::directory();
        let mut template = new_template(&dir, "New Macro");
        template.enabled = false;
        match save(&dir, None, &template) {
            Ok(file_name) => {
                self.notice = Some((format!("Created {}.", template.name), true));
                self.reload(true);
                self.selected = Some(file_name.clone());
                self.load_editor(&file_name);
            }
            Err(error) => self.notice = Some((error, false)),
        }
    }

    fn toggle(&mut self, file_name: &str, enabled: bool) {
        let dir = Self::directory();
        let mut macro_ = match infered_macros::read(&dir, file_name) {
            Ok(macro_) => macro_,
            Err(error) => {
                self.notice = Some((error, false));
                return;
            }
        };
        if macro_.enabled == enabled {
            return;
        }
        macro_.enabled = enabled;
        match save(&dir, Some(file_name), &macro_) {
            Ok(new_file) => {
                self.notice = Some((
                    format!(
                        "{} {}.",
                        macro_.name,
                        if enabled { "enabled" } else { "disabled" }
                    ),
                    true,
                ));
                 
                self.reload(true);
                if new_file != file_name {
                    self.selected = Some(new_file.clone());
                    self.load_editor(&new_file);
                } else if self.selected.as_deref() == Some(file_name) {
                    self.load_editor(file_name);
                }
            }
            Err(error) => self.notice = Some((error, false)),
        }
    }

    fn duplicate_entry(&mut self, file_name: &str) {
        match duplicate(&Self::directory(), file_name) {
            Ok(new_file) => {
                self.notice = Some((format!("Duplicated as {new_file}."), true));
                self.reload(true);
                self.selected = Some(new_file.clone());
                self.load_editor(&new_file);
            }
            Err(error) => self.notice = Some((error, false)),
        }
    }

    fn delete_confirmed(&mut self, file_name: &str) {
        match delete(&Self::directory(), file_name) {
            Ok(()) => {
                self.notice = Some((format!("Deleted {file_name}."), true));
                if self.selected.as_deref() == Some(file_name) {
                    self.selected = None;
                    self.editor = None;
                }
                self.reload(true);
                self.confirm_delete = None;
            }
            Err(error) => self.notice = Some((error, false)),
        }
    }

    fn import_from(&mut self, path: &Path) {
        match import(&Self::directory(), path) {
            Ok(file) => {
                self.notice = Some((format!("Imported {file}."), true));
                self.reload(true);
                self.selected = Some(file.clone());
                self.load_editor(&file);
            }
            Err(error) => self.notice = Some((error, false)),
        }
    }

    fn save_editor(&mut self) {
        let Some(mut editor) = self.editor.take() else {
            return;
        };
        let dir = Self::directory();
        let result: Result<(Macro, String), String> = if editor.tab == Tab::Json {
             
             
            match infered_macros::parse_text(&editor.raw) {
                Ok(macro_) => {
                    let issues = validate(&macro_);
                    if !issues.is_empty() {
                        editor.issues = issues
                            .iter()
                            .map(|issue| format!("{}: {}", issue.path, issue.message))
                            .collect();
                        Err("fix the validation errors before saving".to_string())
                    } else {
                        save_text(&dir, Some(&editor.file_name), &editor.raw)
                    }
                }
                Err(error) => {
                    editor.raw_error = Some(error.clone());
                    Err(error)
                }
            }
        } else {
            match serde_json::from_value::<Macro>(editor.doc.clone()) {
                Ok(macro_) => {
                    let issues = validate(&macro_);
                    if !issues.is_empty() {
                        editor.issues = issues
                            .iter()
                            .map(|issue| format!("{}: {}", issue.path, issue.message))
                            .collect();
                        Err("fix the validation errors before saving".to_string())
                    } else {
                        save(&dir, Some(&editor.file_name), &macro_).map(|file| (macro_, file))
                    }
                }
                Err(error) => Err(format!("document: {error}")),
            }
        };
        match result {
            Ok((macro_, file_name)) => {
                editor.file_name = file_name.clone();
                editor.raw = serde_json::to_string_pretty(&macro_).unwrap_or_default();
                editor.saved = editor.raw.clone();
                editor.raw_error = None;
                editor.validate();
                editor.buffers.clear();
                self.selected = Some(file_name);
                self.notice = Some((format!("Saved {}.", macro_.name), true));
                self.editor = Some(editor);
                self.reload(true);
            }
            Err(error) => {
                self.notice = Some((error, false));
                self.editor = Some(editor);
            }
        }
    }
}

#[cfg(test)]
impl MacrosPage {
     
     
    pub(crate) fn stub_library(&mut self, rows: &[(&str, &str, bool, usize)]) {
        self.entries = rows
            .iter()
            .enumerate()
            .map(|(index, (name, trigger, enabled, actions))| Entry {
                file_name: format!("{}.macro", name.to_lowercase().replace(' ', "_")),
                id: name.to_lowercase().replace(' ', "_"),
                name: (*name).to_string(),
                description: format!("{name} macro"),
                triggers: vec![(*trigger).to_string()],
                enabled: *enabled,
                modified: 1_700_000_000 - index as u64,
                action_count: *actions,
                error: None,
                warning: if *actions == 0 {
                    Some("no actions".to_string())
                } else {
                    None
                },
            })
            .collect();
        self.selected = self.entries.first().map(|entry| entry.file_name.clone());
        self.opened_at = -10.0;
        self.last_scan = 1.0e9;
    }

     
    pub(crate) fn stub_confirm(&mut self, file: &str) {
        self.confirm_delete = Some(file.to_string());
    }

     
     
    pub(crate) fn stub_editor(&mut self) {
        let actions: Vec<Json> = ["wait", "key", "release_inputs"]
            .iter()
            .filter_map(|kind| typed_default(kind, "actions"))
            .collect();
        let doc = serde_json::json!({
            "format_version": 1,
            "id": "strafe_combo",
            "name": "Strafe Combo",
            "description": "Holds a strafe while the sword swings",
            "key": "F8",
            "enabled": true,
            "cooldown_ms": 250,
            "triggers": [trigger_default("key_press")],
            "conditions": [condition_default("compare")],
            "actions": actions,
            "limits": { "max_runtime_ms": 4000, "max_iterations": 24, "on_error": "stop" },
            "variables": { "reach": 3.0 }
        });
        let raw = serde_json::to_string_pretty(&doc).unwrap_or_default();
        let mut editor = Editor {
            file_name: "strafe_combo.macro".to_string(),
            doc,
            raw: raw.clone(),
            saved: raw,
            tab: Tab::Visual,
            ..Editor::default()
        };
        editor.validate();
        self.editor = Some(editor);
        self.selected = Some("strafe_combo.macro".to_string());
    }
}

fn directory_fingerprint(dir: &Path) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut files: Vec<(String, u64, u64)> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("macro") {
                return None;
            }
            let meta = entry.metadata().ok()?;
            let modified = meta
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_nanos() as u64)
                .unwrap_or(0);
            Some((
                entry.file_name().to_string_lossy().into_owned(),
                meta.len(),
                modified,
            ))
        })
        .collect();
    files.sort();
    files.hash(&mut hasher);
    hasher.finish()
}

 

 
pub(super) fn page(gui: &mut InjectorGui, ui: &mut Ui) {
    let now = ui.input(|input| input.time);
    gui.macros_page.poll(now);
    let elapsed = (now - gui.macros_page.opened_at).max(0.0);
    let eased = ease_out_cubic((elapsed / 0.34).clamp(0.0, 1.0) as f32);
    let (area, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());

    let loaded = gui
        .macros_page
        .entries
        .iter()
        .filter(|entry| entry.error.is_none())
        .count();
    let active = gui
        .macros_page
        .entries
        .iter()
        .filter(|entry| entry.enabled && entry.error.is_none())
        .count();
    let broken = gui.macros_page.entries.len() - loaded;

    let mut create = header(
        ui,
        area,
        &gui.macros_icon,
        HeaderCounts {
            loaded,
            active,
            broken,
        },
        now,
        elapsed,
    );

    let top = area.top() + HEADER_H + 10.0;
    let height = (area.bottom() - top).max(80.0);
    let lib_rect = Rect::from_min_size(pos2(area.left(), top), vec2(LIBRARY_W, height));
    let edit_rect = Rect::from_min_size(
        pos2(area.left() + LIBRARY_W + GAP, top),
        vec2((area.width() - LIBRARY_W - GAP).max(180.0), height),
    );

    {
        let mut lib = ui.new_child(
            UiBuilder::new()
                .max_rect(lib_rect)
                .layout(Layout::top_down(Align::Min)),
        );
        lib.set_opacity(eased);
        create |= library_column(gui, &mut lib, now);
    }
    {
        let mut edit = ui.new_child(
            UiBuilder::new()
                .max_rect(edit_rect)
                .layout(Layout::top_down(Align::Min)),
        );
        edit.set_opacity(eased);
        editor_column(gui, &mut edit, now);
    }

     
     
    let open = gui.macros_page.confirm_delete.is_some();
    let appear = ui
        .ctx()
        .animate_bool_with_time(Id::new("macro_delete_modal"), open, 0.18);
    confirmation(gui, ui, area, now, appear);

    if create {
        gui.macros_page.create();
    }
}

 

 
struct HeaderCounts {
    loaded: usize,
    active: usize,
    broken: usize,
}

 
 
fn header(
    ui: &Ui,
    area: Rect,
    icon: &TextureHandle,
    counts: HeaderCounts,
    now: f64,
    elapsed: f64,
) -> bool {
    let HeaderCounts {
        loaded,
        active,
        broken,
    } = counts;
    let draw = ease_out_cubic((elapsed / 0.42).clamp(0.0, 1.0) as f32);
    let mut painter = ui.painter().clone();
    painter.set_opacity(draw);

     
    let mark = Rect::from_min_size(pos2(area.left() + 1.0, area.top() + 2.0), Vec2::splat(28.0));
    radial_glow(
        &painter,
        mark.center(),
        22.0,
        Palette::ACCENT,
        0.10 + 0.05 * pulse(now, 3.6),
    );
    painter.image(
        icon.id(),
        mark.shrink(3.0),
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
        Color32::WHITE,
    );

    let text_x = mark.right() + 10.0;
    tracked(
        &painter,
        "MACROS",
        16.0,
        Palette::TEXT,
        Align::Min,
        text_x,
        area.top() + 13.0,
    );

     
     
    let summary = if broken > 0 {
        format!(
            "{} MACROS  ·  {} UNREADABLE",
            count_up(loaded, elapsed),
            count_up(broken, elapsed)
        )
    } else {
        format!(
            "{} MACROS  ·  {} ACTIVE",
            count_up(loaded, elapsed),
            count_up(active, elapsed)
        )
    };
    let status = if broken > 0 {
        Palette::ERR
    } else if active > 0 {
        Palette::OK
    } else {
        Palette::ACCENT
    };
    painter.circle_filled(
        pos2(text_x + 2.0, area.top() + 30.0),
        2.0,
        with_alpha(status, 0.9),
    );
    tracked(
        &painter,
        &summary,
        7.5,
        with_alpha(Palette::TEXT_DIM, 0.85),
        Align::Min,
        text_x + 10.0,
        area.top() + 30.0,
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

     
    let pill = Rect::from_min_size(
        pos2(area.right() - 2.0 - NEW_PILL.x, area.top() + 8.0),
        NEW_PILL,
    );
    let response = ui.interact(pill, Id::new("macros_new_pill"), Sense::click());
    let hover =
        ui.ctx()
            .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.16);
    let press = ui.ctx().animate_bool_with_time(
        response.id.with("press"),
        response.is_pointer_button_down_on(),
        0.08,
    );
    let appear = ease_out_back(((elapsed - 0.10) / 0.34).clamp(0.0, 1.0) as f32);
    let drawn = pill.translate(vec2(16.0 * (1.0 - appear), 0.0));
    let mut pill_painter = painter.clone();
    pill_painter.set_opacity(draw * appear.clamp(0.0, 1.0));
    glossy_pill(
        &pill_painter,
        drawn,
        PillStyle {
            label: "NEW MACRO",
            icon: PillIcon::Plus,
            emphasis: 1.0,
        },
        hover,
        press,
        now,
    );

    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response.clicked()
}

 

fn library_column(gui: &mut InjectorGui, ui: &mut Ui, now: f64) -> bool {
    ui.spacing_mut().item_spacing = vec2(0.0, 5.0);
    search_field(ui, &mut gui.macros_page.search);
    toolbar_row(gui, ui, now);

    if let Some((message, ok)) = gui.macros_page.notice.clone() {
        notice_strip(ui, &message, ok);
    }

    let query = gui.macros_page.search.trim().to_lowercase();
    let mut entries = gui.macros_page.entries.clone();
    if !query.is_empty() {
        entries.retain(|entry| {
            entry.name.to_lowercase().contains(&query)
                || entry.id.to_lowercase().contains(&query)
                || entry.description.to_lowercase().contains(&query)
                || entry
                    .triggers
                    .iter()
                    .any(|trigger| trigger.to_lowercase().contains(&query))
        });
    }
    match gui.macros_page.sort {
        SortBy::Name => entries.sort_by_key(|entry| entry.name.to_lowercase()),
        SortBy::Modified => entries.sort_by_key(|entry| std::cmp::Reverse(entry.modified)),
        SortBy::Triggers => entries.sort_by_key(|entry| std::cmp::Reverse(entry.triggers.len())),
        SortBy::Enabled => entries.sort_by_key(|entry| std::cmp::Reverse(entry.enabled)),
    }

    if entries.is_empty() {
        return empty_state(ui, query.is_empty(), now, gui.macros_page.opened_at);
    }

    let mut select = None;
    let mut toggle = None;
    let mut duplicate = None;
    let mut export_this = None;
    let mut delete_this = None;
    let selected = gui.macros_page.selected.clone();
    let opened_at = gui.macros_page.opened_at;
    ScrollArea::vertical()
        .id_salt("macro_library")
        .auto_shrink([false, false])
        .max_height((ui.available_height() - 4.0).max(40.0))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 6.0);
            for (index, entry) in entries.iter().enumerate() {
                let (card, toggled) = macro_card(
                    ui,
                    entry,
                    selected.as_deref() == Some(&entry.file_name),
                    index,
                    now,
                    opened_at,
                );
                if card.clicked() {
                    select = Some(entry.file_name.clone());
                }
                if toggled {
                    toggle = Some((entry.file_name.clone(), !entry.enabled));
                }
                card.context_menu(|ui| {
                    if ui.button("Duplicate").clicked() {
                        duplicate = Some(entry.file_name.clone());
                        ui.close_menu();
                    }
                    if ui.button("Export…").clicked() {
                        export_this = Some(entry.file_name.clone());
                        ui.close_menu();
                    }
                    if ui.button("Delete…").clicked() {
                        delete_this = Some(entry.file_name.clone());
                        ui.close_menu();
                    }
                });
            }
            ui.add_space(4.0);
        });

    if let Some(file) = select {
        gui.macros_page.select(&file);
    }
    if let Some((file, enabled)) = toggle {
        gui.macros_page.toggle(&file, enabled);
    }
    if let Some(file) = duplicate {
        gui.macros_page.duplicate_entry(&file);
    }
    if let Some(file) = export_this {
        export_macro(gui, &file);
    }
    if let Some(file) = delete_this {
        gui.macros_page.confirm_delete = Some(file);
    }
    false
}

 
 
fn search_field(ui: &mut Ui, text: &mut String) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::hover());
    let painter = ui.painter().clone();
    let hovered = ui.rect_contains_pointer(rect);
    let clear = (!text.is_empty()).then(|| {
        Rect::from_center_size(
            pos2(rect.right() - 14.0, rect.center().y),
            Vec2::splat(18.0),
        )
    });
    let clear_hot = clear.is_some_and(|clear| ui.rect_contains_pointer(clear));
    painter.rect_filled(
        rect,
        Rounding::same(8.0),
        with_alpha(Palette::CARD, if hovered { 0.78 } else { 0.58 }),
    );
    painter.rect_stroke(
        rect,
        Rounding::same(8.0),
        Stroke::new(
            1.0_f32,
            with_alpha(
                Palette::TEXT_DIM,
                0.14 + 0.10 * if hovered { 1.0 } else { 0.0 },
            ),
        ),
    );
    let lens = pos2(rect.left() + 12.0, rect.center().y - 0.5);
    let glass = with_alpha(Palette::TEXT_DIM, 0.72);
    painter.circle_stroke(lens, 4.2, Stroke::new(1.2_f32, glass));
    painter.line_segment(
        [lens + vec2(3.0, 3.0), lens + vec2(6.2, 6.2)],
        Stroke::new(1.2_f32, glass),
    );

    let right = rect.right() - if clear.is_some() { 28.0 } else { 8.0 };
    let edit_rect = Rect::from_min_max(
        pos2(rect.left() + 24.0, rect.top() + 3.0),
        pos2(right, rect.bottom() - 3.0),
    );
    let response = ui.put(
        edit_rect,
        TextEdit::singleline(text)
            .frame(false)
            .hint_text("Search macros")
            .margin(Margin::ZERO),
    );
    if response.has_focus() {
        painter.rect_stroke(
            rect,
            Rounding::same(8.0),
            Stroke::new(1.0_f32, with_alpha(Palette::ACCENT, 0.55)),
        );
    }
    if let Some(clear) = clear {
        let response = ui.interact(clear, Id::new("macro_search_clear"), Sense::click());
        if clear_hot {
            painter.circle_filled(clear.center(), 8.0, with_alpha(Palette::TEXT_DIM, 0.16));
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        let ink = with_alpha(Palette::TEXT, if clear_hot { 0.95 } else { 0.60 });
        for (a, b) in [
            (vec2(-3.0, -3.0), vec2(3.0, 3.0)),
            (vec2(3.0, -3.0), vec2(-3.0, 3.0)),
        ] {
            painter.line_segment(
                [clear.center() + a, clear.center() + b],
                Stroke::new(1.2_f32, ink),
            );
        }
        if response.clicked() {
            text.clear();
            response.request_focus();
        }
    }
}

 
 
fn toolbar_row(gui: &mut InjectorGui, ui: &mut Ui, now: f64) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 5.0;
        sort_chip(
            ui,
            &mut gui.macros_page.sort,
            &mut gui.macros_page.sort_open,
            100.0,
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            let import = icon_button(ui, ToolIcon::Import, "macro_import", now);
            if import.clicked() {
                if let Some(path) =
                    super::pick_file("Macro files (*.macro)\0*.macro\0All files (*.*)\0*.*\0\0")
                {
                    gui.macros_page.import_from(&path);
                }
            }
            import.on_hover_text("Import a .macro file");
            let refresh = icon_button(ui, ToolIcon::Refresh, "macro_refresh", now);
            if refresh.clicked() {
                gui.macros_page.reload(true);
            }
            refresh.on_hover_text("Rescan the macro folder");
            let folder = icon_button(ui, ToolIcon::Folder, "macro_folder", now);
            if folder.clicked() {
                open_folder(&MacrosPage::directory());
            }
            folder.on_hover_text("Open the macro folder");
        });
    });
}

 
 
fn sort_chip(ui: &mut Ui, sort: &mut SortBy, open: &mut bool, width: f32) {
    let (rect, response) = ui.allocate_exact_size(vec2(width, 24.0), Sense::click());
    let response = response.on_hover_text("Sort the library");
    let id = response.id;
    let hover = ui
        .ctx()
        .animate_bool_with_time(id.with("hover"), response.hovered(), 0.14);
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        Rounding::same(7.0),
        with_alpha(Palette::CARD, 0.55 + 0.22 * hover),
    );
    painter.rect_stroke(
        rect,
        Rounding::same(7.0),
        Stroke::new(
            1.0_f32,
            with_alpha(
                if *open || hover > 0.5 {
                    Palette::ACCENT
                } else {
                    Palette::TEXT_DIM
                },
                0.16 + 0.26 * hover + if *open { 0.20 } else { 0.0 },
            ),
        ),
    );
     
    let ink = with_alpha(Palette::TEXT_DIM, 0.85 + 0.15 * hover);
    for (index, length) in [9.0_f32, 6.5, 4.0].into_iter().enumerate() {
        let y = rect.center().y - 5.0 + index as f32 * 4.5;
        painter.line_segment(
            [
                pos2(rect.left() + 9.0, y),
                pos2(rect.left() + 9.0 + length, y),
            ],
            Stroke::new(1.3_f32, ink),
        );
    }
    let label_x = rect.left() + 26.0;
    tracked(
        painter,
        &sort.label().to_ascii_uppercase(),
        7.0,
        with_alpha(Palette::TEXT, 0.72 + 0.22 * hover),
        Align::Min,
        label_x,
        rect.center().y,
    );
     
    let chevron = pos2(
        rect.right() - 10.0,
        rect.center().y + if *open { 1.2 } else { -1.2 },
    );
    let down = if *open { -1.0_f32 } else { 1.0 };
    painter.line_segment(
        [
            chevron + vec2(-3.2, -1.4 * down),
            chevron + vec2(0.0, 1.6 * down),
        ],
        Stroke::new(1.2_f32, ink),
    );
    painter.line_segment(
        [
            chevron + vec2(3.2, -1.4 * down),
            chevron + vec2(0.0, 1.6 * down),
        ],
        Stroke::new(1.2_f32, ink),
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    if response.clicked() {
        *open = !*open;
    }
    if !*open {
        return;
    }

    let ctx = ui.ctx().clone();
    let mut chosen = None;
    let menu = egui::Area::new(Id::new("macro_sort_menu"))
        .order(Order::Foreground)
        .fixed_pos(pos2(rect.left(), rect.bottom() + 4.0))
        .show(&ctx, |ui| {
            egui::Frame::none()
                .fill(Palette::BG)
                .stroke(Stroke::new(1.0_f32, with_alpha(Palette::ACCENT, 0.30)))
                .rounding(Rounding::same(9.0))
                .inner_margin(Margin::same(4.0))
                .show(ui, |ui| {
                    ui.set_width(width);
                    ui.spacing_mut().item_spacing.y = 2.0;
                    for option in SortBy::ALL {
                        let (row, response) =
                            ui.allocate_exact_size(vec2(width - 8.0, 20.0), Sense::click());
                        if response.hovered() {
                            ui.painter().rect_filled(
                                row,
                                Rounding::same(6.0),
                                with_alpha(Palette::ACCENT, 0.12),
                            );
                            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                        }
                        let active = *sort == option;
                        if active {
                            ui.painter().rect_filled(
                                Rect::from_min_size(
                                    pos2(row.left() + 1.0, row.center().y - 4.0),
                                    vec2(2.0, 8.0),
                                ),
                                Rounding::same(1.0),
                                Palette::ACCENT,
                            );
                        }
                        ui.painter().text(
                            pos2(row.left() + 10.0, row.center().y),
                            Align2::LEFT_CENTER,
                            option.label(),
                            FontId::proportional(9.5),
                            if active {
                                Palette::TEXT
                            } else {
                                with_alpha(Palette::TEXT_DIM, 0.9)
                            },
                        );
                        if response.clicked() {
                            chosen = Some(option);
                        }
                    }
                });
        });
    if ctx.input(|input| input.pointer.any_pressed()) {
        let pointer = ctx.pointer_interact_pos();
        let inside =
            pointer.is_some_and(|point| menu.response.rect.contains(point) || rect.contains(point));
        if !inside {
            *open = false;
        }
    }
    if let Some(option) = chosen {
        *sort = option;
        *open = false;
    }
}

 
#[derive(Clone, Copy)]
enum ToolIcon {
    Folder,
    Refresh,
    Import,
}

 
 
fn icon_button(ui: &mut Ui, icon: ToolIcon, salt: &str, now: f64) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
    let id = Id::new(salt).with(response.id);
    let hover = ui
        .ctx()
        .animate_bool_with_time(id.with("hover"), response.hovered(), 0.14);
    let press = ui.ctx().animate_bool_with_time(
        id.with("press"),
        response.is_pointer_button_down_on(),
        0.08,
    );
    let body = rect.shrink(0.5).translate(vec2(0.0, press));
    let painter = ui.painter();
    painter.rect_filled(
        body,
        Rounding::same(7.0),
        with_alpha(Palette::CARD, 0.55 + 0.28 * hover),
    );
    painter.rect_stroke(
        body,
        Rounding::same(7.0),
        Stroke::new(
            1.0_f32,
            with_alpha(
                if hover > 0.5 {
                    Palette::ACCENT
                } else {
                    Palette::TEXT_DIM
                },
                0.14 + 0.30 * hover,
            ),
        ),
    );
    let ink = with_alpha(
        lerp_color(Palette::TEXT_DIM, Palette::TEXT, hover),
        0.86 + 0.12 * hover,
    );
    let stroke = Stroke::new(1.3_f32, ink);
    let c = body.center();
    match icon {
        ToolIcon::Folder => {
            let points = vec![
                c + vec2(-6.0, -4.4),
                c + vec2(-1.4, -4.4),
                c + vec2(-0.4, -3.0),
                c + vec2(6.0, -3.0),
                c + vec2(6.0, 4.4),
                c + vec2(-6.0, 4.4),
                c + vec2(-6.0, -4.4),
            ];
            painter.add(Shape::line(points, stroke));
        }
        ToolIcon::Import => {
            let drop = 1.0 * hover;
            painter.add(Shape::line(
                vec![
                    c + vec2(-5.0, 2.6),
                    c + vec2(-5.0, 4.6),
                    c + vec2(5.0, 4.6),
                    c + vec2(5.0, 2.6),
                ],
                stroke,
            ));
            painter.line_segment([c + vec2(0.0, -5.0), c + vec2(0.0, 1.8 + drop)], stroke);
            painter.line_segment(
                [c + vec2(-3.0, -1.0 + drop), c + vec2(0.0, 2.0 + drop)],
                stroke,
            );
            painter.line_segment(
                [c + vec2(3.0, -1.0 + drop), c + vec2(0.0, 2.0 + drop)],
                stroke,
            );
        }
        ToolIcon::Refresh => {
            let angle = 0.9 * hover + (now as f32 * 0.6).sin() * 0.05;
            arc_stroke(painter, c, 5.0, angle.to_degrees() + 40.0, 280.0, stroke);
            let end = angle + (40.0_f32 + 280.0).to_radians();
            let tip = c + vec2(end.cos(), end.sin()) * 5.0;
            let tangent = end + std::f32::consts::FRAC_PI_2;
            let wing = vec2(tangent.cos(), tangent.sin()) * 3.0;
            painter.line_segment([tip, tip - wing + vec2(-1.4, 0.0)], stroke);
            painter.line_segment([tip, tip - wing + vec2(0.0, 1.4)], stroke);
        }
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response
}

fn arc_stroke(
    painter: &egui::Painter,
    centre: Pos2,
    radius: f32,
    start_deg: f32,
    sweep_deg: f32,
    stroke: Stroke,
) {
    const SEGMENTS: usize = 18;
    let mut points = Vec::with_capacity(SEGMENTS + 1);
    for step in 0..=SEGMENTS {
        let angle = (start_deg + sweep_deg * step as f32 / SEGMENTS as f32).to_radians();
        points.push(centre + vec2(angle.cos(), angle.sin()) * radius);
    }
    painter.add(Shape::line(points, stroke));
}

 
 
fn notice_strip(ui: &mut Ui, message: &str, ok: bool) {
    let color = if ok { Palette::OK } else { Palette::ERR };
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, Rounding::same(8.0), with_alpha(color, 0.07));
    painter.rect_stroke(
        rect,
        Rounding::same(8.0),
        Stroke::new(1.0_f32, with_alpha(color, 0.28)),
    );
    painter.circle_filled(
        pos2(rect.left() + 11.0, rect.center().y),
        2.4,
        with_alpha(color, 0.95),
    );
    let galley = elided(
        ui,
        message,
        9.5,
        with_alpha(Palette::TEXT, 0.90),
        (rect.width() - 74.0).max(30.0),
    );
    painter.galley(
        pos2(rect.left() + 20.0, rect.center().y - galley.size().y * 0.5),
        galley,
        Palette::TEXT,
    );
    tracked(
        painter,
        if ok { "OK" } else { "FAILED" },
        6.5,
        with_alpha(color, 0.85),
        Align::Max,
        rect.right() - 10.0,
        rect.center().y,
    );
}

 

 
 
 
fn macro_card(
    ui: &mut Ui,
    entry: &Entry,
    selected: bool,
    index: usize,
    now: f64,
    opened_at: f64,
) -> (egui::Response, bool) {
    let t = ((now - opened_at - index as f64 * 0.045) / 0.38).clamp(0.0, 1.0) as f32;
    if t <= 0.0 {
        ui.ctx().request_repaint();
        let (slot, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), CARD_H), Sense::hover());
        let _ = slot;
        return (response, false);
    }
    let appear = ease_out_back(t);
    let fade = ease_out_cubic((t / 0.60).clamp(0.0, 1.0));

    let (slot, _) = ui.allocate_exact_size(
        vec2((ui.available_width() - 2.0).max(80.0), CARD_H),
        Sense::hover(),
    );
    let id = Id::new(("macro_row", entry.file_name.as_str()));
    let response = ui.interact(slot, id, Sense::click());
    let hover =
        ui.ctx()
            .animate_bool_with_time(id.with("hover"), ui.rect_contains_pointer(slot), 0.16);
    let press = ui.ctx().animate_bool_with_time(
        id.with("press"),
        response.is_pointer_button_down_on(),
        0.09,
    );
    let on = ui.ctx().animate_bool_with_time(
        id.with("on"),
        entry.enabled && entry.error.is_none(),
        0.22,
    );

    let switch = Rect::from_center_size(
        pos2(slot.right() - 14.0 - 17.0, slot.center().y),
        vec2(34.0, 18.0),
    );
    let switch_response = ui.interact(switch, id.with("toggle"), Sense::click());
    let switch_hover = ui.ctx().animate_bool_with_time(
        switch_response.id.with("hover"),
        switch_response.hovered(),
        0.12,
    );

    let rect = slot
        .translate(vec2(-18.0 * (1.0 - appear), 7.0 * (1.0 - appear)))
        .translate(vec2(0.0, -1.5 * hover + 1.0 * press));
    let mut painter = ui.painter().clone();
    painter.set_opacity(fade);

     
    painter.rect_filled(
        rect.translate(vec2(0.0, 2.5)).expand(0.5),
        Rounding::same(13.0),
        with_alpha(Color32::BLACK, 0.28 + 0.10 * hover),
    );
    if hover > 0.01 {
        for (spread, alpha) in [(4.0_f32, 0.040_f32), (1.5, 0.060)] {
            painter.rect_filled(
                rect.expand(spread),
                Rounding::same(14.0),
                with_alpha(Palette::ACCENT, alpha * hover),
            );
        }
    }
    let base = lerp_color(
        Palette::CARD,
        Palette::ACCENT_DEEP,
        if selected { 0.22 } else { 0.0 },
    );
    painter.rect_filled(
        rect,
        Rounding::same(10.0),
        lerp_color(base, Palette::CARD_HOVER, hover),
    );
    top_light_rounded(&painter, rect, 10.0, 0.055 + 0.025 * hover);
    if press > 0.01 {
        painter.rect_filled(
            rect,
            Rounding::same(10.0),
            with_alpha(Palette::ACCENT_BRIGHT, 0.05 * press),
        );
    }
    if hover > 0.01 {
        if let Some(pointer) = ui.ctx().pointer_hover_pos() {
            let focus = pos2(
                pointer.x.clamp(rect.left() + 10.0, rect.right() - 10.0),
                pointer.y.clamp(rect.top() + 7.0, rect.bottom() - 7.0),
            );
            radial_glow(
                &painter.with_clip_rect(rect.shrink(1.0)),
                focus,
                70.0,
                Palette::ACCENT,
                0.055 * hover,
            );
        }
    }

    let accent = if entry.error.is_some() {
        Palette::ERR
    } else if entry.enabled {
        Palette::ACCENT
    } else {
        Palette::TEXT_DIM
    };
    painter.rect_stroke(
        rect,
        Rounding::same(10.0),
        Stroke::new(
            1.0_f32,
            with_alpha(
                accent,
                if selected { 0.46 } else { 0.14 + 0.28 * hover } + 0.14 * press,
            ),
        ),
    );
    painter.line_segment(
        [
            pos2(rect.left() + 12.0, rect.top() + 1.0),
            pos2(rect.right() - 12.0, rect.top() + 1.0),
        ],
        Stroke::new(1.0_f32, with_alpha(Color32::WHITE, 0.05 + 0.03 * hover)),
    );
     
     
    painter.rect_filled(
        Rect::from_center_size(
            pos2(rect.left() + 1.5, rect.center().y),
            vec2(2.5, if selected { 26.0 } else { 12.0 + 18.0 * hover }),
        ),
        Rounding::same(1.5),
        with_alpha(
            if entry.error.is_some() {
                Palette::ERR
            } else {
                accent
            },
            if selected { 0.85 } else { 0.18 + 0.60 * hover },
        ),
    );

     
    let well = Rect::from_center_size(
        pos2(rect.left() + 11.0 + 15.0, rect.center().y),
        Vec2::splat(30.0 + 0.6 * hover),
    );
    painter.rect_filled(
        well,
        Rounding::same(8.0),
        with_alpha(
            lerp_color(Color32::WHITE, Palette::ACCENT, on),
            0.055 + 0.035 * hover,
        ),
    );
    painter.rect_stroke(
        well,
        Rounding::same(8.0),
        Stroke::new(
            1.0_f32,
            with_alpha(
                lerp_color(Palette::TEXT_DIM, Palette::ACCENT, on.max(hover * 0.5)),
                0.20 + 0.28 * hover,
            ),
        ),
    );
    macro_glyph(&painter, well, on, hover, now);

     
    let text_x = well.right() + 9.0;
    let width = (switch.left() - 10.0 - text_x).max(24.0);
    let title = elided(
        ui,
        &entry.name,
        11.5,
        with_alpha(Palette::TEXT, 0.86 + 0.14 * hover),
        width,
    );
    painter.galley(
        pos2(text_x, rect.center().y - 8.0 - title.size().y * 0.5),
        title,
        Palette::TEXT,
    );
     
     
    let trigger = if entry.error.is_some() {
        "unreadable file".to_string()
    } else if entry.triggers.is_empty() {
        "manual".to_string()
    } else {
        entry.triggers.join(" · ")
    };
    let meta = if entry.error.is_some() {
        "fix the file to load it".to_string()
    } else if let Some(warning) = &entry.warning {
        warning.clone()
    } else {
        format!("{} actions", entry.action_count)
    };
    let detail = elided(
        ui,
        &format!("{trigger}  ·  {meta}"),
        8.5,
        with_alpha(
            if entry.error.is_some() {
                Palette::ERR
            } else {
                Palette::TEXT_DIM
            },
            0.74 + 0.16 * hover,
        ),
        width,
    );
    painter.galley(
        pos2(text_x, rect.center().y + 8.0 - detail.size().y * 0.5),
        detail,
        Palette::TEXT_DIM,
    );

    if entry.error.is_none() {
        paint_toggle(&painter, switch, on, switch_hover);
    } else {
        painter.circle_filled(
            switch.center(),
            2.5,
            with_alpha(Palette::ERR, 0.7 + 0.3 * switch_hover),
        );
    }
    sweep(
        &painter,
        rect.shrink(1.0),
        now + index as f64 * 1.1,
        6.4,
        0.02,
    );

    if response.hovered() || switch_response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    let toggled = switch_response.clicked() && entry.error.is_none();
    (response, toggled)
}

 
 
fn macro_glyph(painter: &egui::Painter, well: Rect, on: f32, hover: f32, now: f64) {
    let centre = well.center();
    let scale = well.width() / 30.0;
    let tint = lerp_color(
        Palette::TEXT_DIM,
        Palette::ACCENT_BRIGHT,
        on.max(hover * 0.55),
    );
    radial_glow(
        painter,
        centre,
        16.0 * scale,
        Palette::ACCENT,
        0.04 * on + 0.05 * hover,
    );
    painter.circle_stroke(
        centre,
        8.0 * scale,
        Stroke::new(1.1_f32 * scale, with_alpha(tint, 0.42 + 0.30 * hover)),
    );
    let radius = (4.0 + 0.5 * hover) * scale;
    let shift = 0.9 * hover * scale;
    painter.add(Shape::convex_polygon(
        vec![
            centre + vec2(-radius * 0.62 + shift, -radius),
            centre + vec2(radius * 1.02 + shift, 0.0),
            centre + vec2(-radius * 0.62 + shift, radius),
        ],
        with_alpha(tint, 0.55 + 0.38 * on),
        Stroke::new(0.0_f32, Color32::TRANSPARENT),
    ));
    if on > 0.01 {
        let angle = now as f32 * 1.7;
        let satellite = centre + vec2(angle.cos(), angle.sin()) * 11.5 * scale;
        painter.circle_filled(
            satellite,
            1.4 * scale,
            with_alpha(Palette::ACCENT_BRIGHT, 0.9 * on),
        );
    }
}

 

 
 
fn empty_state(ui: &mut Ui, empty_library: bool, now: f64, opened_at: f64) -> bool {
    let elapsed = (now - opened_at).max(0.0);
    let appear = ease_out_cubic(((elapsed - 0.08) / 0.36).clamp(0.0, 1.0) as f32);
    let height = (ui.available_height() - 2.0).max(110.0);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let id = Id::new("macro_empty");
    let sense = if empty_library {
        Sense::click()
    } else {
        Sense::hover()
    };
    let response = ui.interact(rect, id, sense);
    let hover = ui.ctx().animate_bool_with_time(
        id.with("hover"),
        empty_library && response.hovered(),
        0.20,
    );
    let press = ui.ctx().animate_bool_with_time(
        id.with("press"),
        response.is_pointer_button_down_on(),
        0.09,
    );

    let mut painter = ui.painter().clone();
    painter.set_opacity(appear);
    painter.rect_filled(
        rect,
        Rounding::same(12.0),
        with_alpha(Color32::WHITE, 0.018 + 0.012 * hover),
    );
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
                0.12 + 0.28 * hover,
            ),
        ),
    );
    let arm = 12.0;
    let ink = Stroke::new(1.5_f32, with_alpha(Palette::ACCENT, 0.22 + 0.50 * hover));
    for (sx, sy) in [(-1.0_f32, -1.0_f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let corner = pos2(
            rect.center().x + sx * (rect.width() * 0.5 - 9.0),
            rect.center().y + sy * (rect.height() * 0.5 - 9.0),
        );
        painter.line_segment([corner, corner + vec2(-sx * arm, 0.0)], ink);
        painter.line_segment([corner, corner + vec2(0.0, -sy * arm)], ink);
    }

     
    let centre = pos2(rect.center().x, rect.top() + rect.height() * 0.34);
    let float = 2.5 * (2.0 * pulse(now, 3.2) - 1.0);
    radial_glow(
        &painter,
        centre + vec2(0.0, float),
        56.0,
        Palette::ACCENT,
        0.045 + 0.055 * hover,
    );
    let well = Rect::from_center_size(centre + vec2(0.0, float), Vec2::splat(42.0 + 3.0 * hover));
    painter.rect_filled(
        well,
        Rounding::same(11.0),
        with_alpha(Palette::ACCENT, 0.07 + 0.05 * hover),
    );
    painter.rect_stroke(
        well,
        Rounding::same(11.0),
        Stroke::new(1.0_f32, with_alpha(Palette::ACCENT, 0.30 + 0.32 * hover)),
    );
    macro_glyph(&painter, well, 0.0, hover, now);

    painter.text(
        pos2(centre.x, centre.y + 42.0),
        Align2::CENTER_CENTER,
        if empty_library {
            "No macros yet"
        } else {
            "Nothing matches"
        },
        FontId::proportional(12.5),
        with_alpha(Palette::TEXT, 0.78 + 0.22 * hover),
    );
    painter.text(
        pos2(centre.x, centre.y + 57.0),
        Align2::CENTER_CENTER,
        if empty_library {
            "Write one here, or drop a .macro into the folder"
        } else {
            "Try another name, description or trigger"
        },
        FontId::proportional(9.0),
        with_alpha(Palette::TEXT_DIM, 0.62 + 0.18 * hover),
    );

    if empty_library {
        let cta = Rect::from_center_size(
            pos2(centre.x, rect.bottom() - 24.0 + 1.0 * press),
            vec2(120.0, 25.0),
        );
        glossy_pill(
            &painter,
            cta,
            PillStyle {
                label: "NEW MACRO",
                icon: PillIcon::Plus,
                emphasis: 1.0,
            },
            hover,
            press,
            now,
        );
        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        response.clicked()
    } else {
        false
    }
}

fn editor_column(gui: &mut InjectorGui, ui: &mut Ui, now: f64) {
    let Some(mut editor) = gui.macros_page.editor.take() else {
        empty_editor(ui);
        return;
    };
    ui.spacing_mut().item_spacing = vec2(5.0, 5.0);

    let mut save_requested = false;
    ui.horizontal(|ui| {
        if let Some(tab) = tab_switch(ui, editor.tab, now) {
            match tab {
                Tab::Visual => {
                    if editor.tab == Tab::Json {
                        if editor.apply_raw() {
                            editor.tab = Tab::Visual;
                        }
                    } else {
                        editor.tab = Tab::Visual;
                    }
                }
                Tab::Json => {
                    if editor.tab == Tab::Visual {
                        editor.resync_raw();
                        editor.raw_error = None;
                        editor.tab = Tab::Json;
                    }
                }
            }
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            let dirty = editor.dirty();
            if glossy_button(
                ui,
                if dirty { "SAVE" } else { "SAVED" },
                70.0,
                24.0,
                dirty,
                now,
            )
            .clicked()
            {
                save_requested = true;
            }
            if history_button(ui, false, !editor.redo.is_empty(), now).clicked() {
                editor.redo();
            }
            if history_button(ui, true, !editor.undo.is_empty(), now).clicked() {
                editor.undo();
            }
        });
    });

    if editor.dirty() {
        ui.horizontal(|ui| {
            let (dot, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
            let beat = pulse(now, 2.2);
            ui.painter().circle_filled(
                dot.center(),
                2.2,
                with_alpha(Palette::ACCENT, 0.55 + 0.40 * beat),
            );
            ui.label(
                RichText::new("unsaved changes")
                    .size(9.0)
                    .color(with_alpha(Palette::ACCENT, 0.85)),
            );
        });
    }

    let before = editor.doc.clone();
    match editor.tab {
        Tab::Json => raw_tab(&mut editor, ui),
        Tab::Visual => visual_tab(&mut editor, ui),
    }
    if editor.doc != before {
        editor.snapshot(before);
        if editor.tab == Tab::Visual {
            editor.resync_raw();
            editor.validate();
        }
    }
    if save_requested {
        gui.macros_page.editor = Some(editor);
        gui.macros_page.save_editor();
        return;
    }
    if !editor.issues.is_empty() {
        paint_issues(ui, &editor.issues);
    }
    if let Some(error) = editor.raw_error.clone() {
        paint_notice(ui, &error, false);
    }
    gui.macros_page.editor = Some(editor);
}

fn empty_editor(ui: &mut Ui) {
    ui.add_space(16.0);
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(
            ui.available_width(),
            (ui.available_height() - 16.0).max(150.0),
        ),
        Sense::hover(),
    );
    let painter = ui.painter();
    painter.rect_filled(rect, Rounding::same(10.0), with_alpha(Palette::CARD, 0.40));
    painter.rect_stroke(
        rect,
        Rounding::same(10.0),
        Stroke::new(1.0_f32, with_alpha(Palette::TEXT_DIM, 0.12)),
    );
    let centre_x = rect.center().x;
    let well = Rect::from_center_size(
        pos2(centre_x, rect.top() + rect.height() * 0.38),
        Vec2::splat(44.0),
    );
    painter.rect_filled(
        well,
        Rounding::same(11.0),
        with_alpha(Palette::ACCENT, 0.06),
    );
    painter.rect_stroke(
        well,
        Rounding::same(11.0),
        Stroke::new(1.0_f32, with_alpha(Palette::ACCENT, 0.24)),
    );
    macro_glyph(painter, well, 0.0, 0.0, ui.input(|input| input.time));
    painter.text(
        pos2(centre_x, well.bottom() + 20.0),
        Align2::CENTER_CENTER,
        "Select a macro to edit it",
        FontId::proportional(12.0),
        with_alpha(Palette::TEXT_DIM, 0.85),
    );
    painter.text(
        pos2(centre_x, well.bottom() + 36.0),
        Align2::CENTER_CENTER,
        "Its triggers, conditions and actions appear here",
        FontId::proportional(9.0),
        with_alpha(Palette::TEXT_DIM, 0.50),
    );
}

 
 
fn tab_switch(ui: &mut Ui, active: Tab, now: f64) -> Option<Tab> {
    let (rect, _) = ui.allocate_exact_size(vec2(132.0, 24.0), Sense::hover());
    let id = Id::new("macro_tabs");
    let slide = ui
        .ctx()
        .animate_bool_with_time(id.with("json"), active == Tab::Json, 0.18);
    let half = (rect.width() - 6.0) * 0.5;
    let visual = Rect::from_min_size(
        pos2(rect.left() + 3.0, rect.top()),
        vec2(half, rect.height()),
    );
    let json = Rect::from_min_size(
        pos2(rect.left() + 3.0 + half, rect.top()),
        vec2(half, rect.height()),
    );
    let visual_response = ui.interact(visual, id.with("visual"), Sense::click());
    let json_response = ui.interact(json, id.with("json_tab"), Sense::click());
    if visual_response.hovered() || json_response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }

    let painter = ui.painter();
    painter.rect_filled(rect, Rounding::same(7.0), with_alpha(Palette::CARD, 0.55));
    painter.rect_stroke(
        rect,
        Rounding::same(7.0),
        Stroke::new(1.0_f32, with_alpha(Palette::TEXT_DIM, 0.14)),
    );
    let pill = Rect::from_min_size(
        pos2(rect.left() + 3.0 + half * slide, rect.top() + 3.0),
        vec2(half, rect.height() - 6.0),
    );
    painter.rect_filled(
        pill,
        Rounding::same(5.0),
        with_alpha(Palette::ACCENT, 0.14 + 0.05 * pulse(now, 3.0)),
    );
    painter.rect_stroke(
        pill,
        Rounding::same(5.0),
        Stroke::new(1.0_f32, with_alpha(Palette::ACCENT, 0.42)),
    );
    for (tab, area, label) in [(Tab::Visual, visual, "VISUAL"), (Tab::Json, json, "JSON")] {
        tracked(
            painter,
            label,
            7.0,
            if tab == active {
                with_alpha(Palette::TEXT, 0.95)
            } else {
                with_alpha(Palette::TEXT_DIM, 0.80)
            },
            Align::Center,
            area.center().x,
            area.center().y,
        );
    }

    if json_response.clicked() {
        Some(Tab::Json)
    } else if visual_response.clicked() {
        Some(Tab::Visual)
    } else {
        None
    }
}

 
 
fn glossy_button(
    ui: &mut Ui,
    label: &str,
    width: f32,
    height: f32,
    emphasis: bool,
    now: f64,
) -> egui::Response {
    let (slot, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let response = ui.interact(slot, Id::new("macro_save"), Sense::click());
    let hover =
        ui.ctx()
            .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.16);
    let press = ui.ctx().animate_bool_with_time(
        response.id.with("press"),
        response.is_pointer_button_down_on(),
        0.08,
    );
    glossy_pill(
        ui.painter(),
        slot,
        PillStyle {
            label,
            icon: PillIcon::None,
            emphasis: if emphasis { 1.0 } else { 0.28 },
        },
        hover,
        press,
        now,
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response
}

 
 
fn history_button(ui: &mut Ui, undo: bool, enabled: bool, _now: f64) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
    let id = Id::new(if undo { "macro_undo" } else { "macro_redo" });
    let hover =
        ui.ctx()
            .animate_bool_with_time(id.with("hover"), response.hovered() && enabled, 0.14);
    let press = ui.ctx().animate_bool_with_time(
        id.with("press"),
        response.is_pointer_button_down_on() && enabled,
        0.08,
    );
    let painter = ui.painter();
    if hover > 0.01 {
        painter.rect_filled(
            rect,
            Rounding::same(7.0),
            with_alpha(Palette::ACCENT, 0.10 * hover),
        );
        painter.rect_stroke(
            rect,
            Rounding::same(7.0),
            Stroke::new(1.0_f32, with_alpha(Palette::ACCENT, 0.25 * hover)),
        );
    }
    let alpha = if enabled { 0.62 + 0.30 * hover } else { 0.22 };
    let ink = with_alpha(lerp_color(Palette::TEXT_DIM, Palette::TEXT, hover), alpha);
    let stroke = Stroke::new(1.3_f32, ink);
    let centre = rect.center() + vec2(0.0, press);
    let flip_x = if undo { -1.0 } else { 1.0 };
    let tilt = 0.16 * hover * flip_x;
    let start = 200.0_f32.to_radians();
    let sweep = 250.0_f32.to_radians();
    let point = |angle: f32| {
        let (sin, cos) = (angle + tilt).sin_cos();
        centre + vec2(cos * flip_x, sin) * 5.0
    };
    let mut points = Vec::with_capacity(21);
    for step in 0..=20 {
        points.push(point(start + sweep * step as f32 / 20.0));
    }
    painter.add(Shape::line(points, stroke));
    let end = start + sweep;
    let tip = point(end);
    let tangent_angle = end + tilt;
    let tangent = vec2(-tangent_angle.sin() * flip_x, tangent_angle.cos());
    let normal = vec2(tangent.y, -tangent.x);
    painter.line_segment([tip, tip - tangent * 2.6 + normal * 2.0], stroke);
    painter.line_segment([tip, tip - tangent * 2.6 - normal * 2.0], stroke);
    if response.hovered() && enabled {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response
}

fn paint_issues(ui: &mut Ui, issues: &[String]) {
    let text = if issues.len() == 1 {
        issues[0].clone()
    } else {
        format!("{} issues · {}", issues.len(), issues[0])
    };
    paint_notice(ui, &text, false);
}

fn visual_tab(editor: &mut Editor, ui: &mut Ui) {
    ScrollArea::vertical()
        .id_salt("macro_visual_editor")
        .auto_shrink([false, false])
        .max_height(ui.available_height())
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 6.0);
            let before = editor.doc.clone();
            metadata(editor, ui);
            if editor.doc != before {
                 
            }
            ui.add_space(2.0);
            section(ui, "TRIGGERS", "what starts the macro");
            let trigger_types = trigger_types();
            array_editor(editor, ui, "triggers", &trigger_types, "trigger");
            ui.add_space(4.0);
            section(ui, "CONDITIONS", "all must hold before a trigger fires");
            let condition_types = vec!["compare", "all", "any", "not"];
            array_editor(editor, ui, "conditions", &condition_types, "condition");
            ui.add_space(4.0);
            section(ui, "ACTIONS", "played back in order");
            let action_types: Vec<&str> = action_type_names();
            array_editor(editor, ui, "actions", &action_types, "action");
            ui.add_space(6.0);
            section(ui, "LIMITS", "safety ceilings for one run");
            limits(editor, ui);
            ui.add_space(4.0);
            section(ui, "VARIABLES", "values shared by actions and conditions");
            variables(editor, ui);
        });
}

fn metadata(editor: &mut Editor, ui: &mut Ui) {
    let mut row = |ui: &mut Ui, label: &str, key: &str| {
        ui.horizontal(|ui| {
            ui.add_sized(
                [70.0, 18.0],
                egui::Label::new(
                    RichText::new(label)
                        .size(9.0)
                        .color(with_alpha(Palette::TEXT_DIM, 0.8)),
                ),
            );
            let initial = editor
                .doc
                .get(key)
                .and_then(Json::as_str)
                .unwrap_or("")
                .to_string();
            let buffer = editor.buffer(key, initial);
            let mut edited = buffer;
            let width = (ui.available_width() - 4.0).max(80.0);
            let response = ui.add_sized(
                [width, 22.0],
                TextEdit::singleline(&mut edited).margin(Margin::symmetric(6.0, 3.0)),
            );
            if response.changed() {
                if let Some(object) = editor.doc.as_object_mut() {
                    object.insert(key.to_string(), Json::String(edited.clone()));
                }
            }
            editor.put_buffer(key, edited);
        });
    };
    row(ui, "Name", "name");
    row(ui, "ID", "id");
    row(ui, "Key", "key");
    ui.horizontal(|ui| {
        ui.add_sized(
            [70.0, 18.0],
            egui::Label::new(
                RichText::new("Description")
                    .size(9.0)
                    .color(with_alpha(Palette::TEXT_DIM, 0.8)),
            ),
        );
        let initial = editor
            .doc
            .get("description")
            .and_then(Json::as_str)
            .unwrap_or("")
            .to_string();
        let buffer = editor.buffer("description", initial);
        let mut edited = buffer;
        let width = (ui.available_width() - 4.0).max(80.0);
        let response = ui.add_sized(
            [width, 22.0],
            TextEdit::singleline(&mut edited)
                .hint_text("shown in the library")
                .margin(Margin::symmetric(6.0, 3.0)),
        );
        if response.changed() {
            if let Some(object) = editor.doc.as_object_mut() {
                object.insert("description".to_string(), Json::String(edited.clone()));
            }
        }
        editor.put_buffer("description", edited);
    });
    ui.horizontal(|ui| {
        let mut enabled = editor
            .doc
            .get("enabled")
            .and_then(Json::as_bool)
            .unwrap_or(false);
        if toggle_switch(ui, Id::new("macro_editor_enabled"), &mut enabled) {
            if let Some(object) = editor.doc.as_object_mut() {
                object.insert("enabled".to_string(), Json::Bool(enabled));
            }
        }
        ui.label(
            RichText::new("Enabled")
                .size(9.0)
                .color(with_alpha(Palette::TEXT_DIM, 0.8)),
        );
        let cooldown = editor
            .doc
            .get("cooldown_ms")
            .and_then(Json::as_u64)
            .unwrap_or(0);
        let initial = format!("{cooldown}");
        let mut buffer = editor.buffer("cooldown_ms", initial);
        ui.add_space(8.0);
        ui.label(
            RichText::new("Cooldown ms")
                .size(9.0)
                .color(with_alpha(Palette::TEXT_DIM, 0.8)),
        );
        let response = ui.add_sized(
            [54.0, 20.0],
            TextEdit::singleline(&mut buffer).margin(Margin::symmetric(4.0, 2.0)),
        );
        if response.changed() {
            if let Ok(value) = buffer.trim().parse::<u64>() {
                if let Some(object) = editor.doc.as_object_mut() {
                    object.insert("cooldown_ms".to_string(), Json::from(value));
                }
            }
        }
        editor.put_buffer("cooldown_ms", buffer);
    });
}

fn limits(editor: &mut Editor, ui: &mut Ui) {
    let mut limits = editor
        .doc
        .get("limits")
        .and_then(Json::as_object)
        .cloned()
        .unwrap_or_default();
    let mut edits = Vec::new();
    ui.horizontal(|ui| {
        for (label, key) in [
            ("Max runtime ms", "max_runtime_ms"),
            ("Max iterations", "max_iterations"),
        ] {
            ui.label(
                RichText::new(label)
                    .size(8.5)
                    .color(with_alpha(Palette::TEXT_DIM, 0.75)),
            );
            let initial = limits.get(key).map(|v| v.to_string()).unwrap_or_default();
            let path = format!("limits.{key}");
            let mut buffer = editor.buffer(&path, initial);
            let response = ui.add_sized(
                [58.0, 20.0],
                TextEdit::singleline(&mut buffer).margin(Margin::symmetric(4.0, 2.0)),
            );
            if response.changed() {
                if let Ok(value) = buffer.trim().parse::<u64>() {
                    edits.push((key.to_string(), Json::from(value)));
                }
            }
            editor.put_buffer(&path, buffer);
            ui.add_space(4.0);
        }
    });
    for (key, value) in edits {
        limits.insert(key, value);
    }
    let policy = limits
        .get("on_error")
        .and_then(Json::as_str)
        .unwrap_or("stop")
        .to_string();
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("On error")
                .size(8.5)
                .color(with_alpha(Palette::TEXT_DIM, 0.75)),
        );
        egui::ComboBox::from_id_salt("macro_on_error")
            .selected_text(&policy)
            .width(84.0)
            .show_ui(ui, |ui| {
                for option in ["stop", "continue"] {
                    if ui.selectable_label(policy == option, option).clicked() {
                        limits.insert("on_error".to_string(), Json::String(option.into()));
                    }
                }
            });
    });
    if let Some(object) = editor.doc.as_object_mut() {
        object.insert("limits".to_string(), Json::Object(limits));
    }
}

fn variables(editor: &mut Editor, ui: &mut Ui) {
    let mut object = editor
        .doc
        .get("variables")
        .and_then(Json::as_object)
        .cloned()
        .unwrap_or_default();
    if object.is_empty() {
        ui.label(
            RichText::new("No variables — add them in the JSON tab")
                .size(8.5)
                .color(with_alpha(Palette::TEXT_DIM, 0.6)),
        );
        return;
    }
    let keys: Vec<String> = object.keys().cloned().collect();
    for key in keys {
        let mut value = object.get(&key).cloned().unwrap_or(Json::Null);
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.add_sized(
                [80.0, 18.0],
                egui::Label::new(
                    RichText::new(&key)
                        .size(9.0)
                        .color(with_alpha(Palette::TEXT_DIM, 0.85)),
                ),
            );
            let mut text = value.to_string();
            let response = ui.add_sized(
                [(ui.available_width() - 4.0).max(60.0), 20.0],
                TextEdit::singleline(&mut text).margin(Margin::symmetric(4.0, 2.0)),
            );
            if response.changed() {
                if let Ok(parsed) = serde_json::from_str::<Json>(&text) {
                    value = parsed;
                    changed = true;
                }
            }
        });
        if changed {
            object.insert(key, value);
        }
        ui.add_space(1.0);
    }
    if let Some(map) = editor.doc.as_object_mut() {
        map.insert("variables".to_string(), Json::Object(object));
    }
}

fn array_editor(editor: &mut Editor, ui: &mut Ui, field: &str, types: &[&str], noun: &str) {
    let mut array = editor
        .doc
        .get(field)
        .and_then(Json::as_array)
        .cloned()
        .unwrap_or_default();
    let mut changed = false;
    let mut remove = None;
    let mut swap = None;
    let mut add = None;

    let length = array.len();
    for (index, item) in array.iter_mut().enumerate() {
        let path = format!("{field}[{index}]");
        let before = item.clone();
        let step = egui::Frame::none()
            .fill(with_alpha(Palette::CARD, 0.46))
            .stroke(Stroke::new(1.0_f32, with_alpha(Palette::TEXT_DIM, 0.10)))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::symmetric(8.0, 6.0))
            .show(ui, |ui| {
                let summary = if noun == "action" {
                    serde_json::from_value::<infered_macros::Action>(item.clone())
                        .map(|action| action_summary(&action))
                        .ok()
                } else {
                    item.get("type")
                        .and_then(Json::as_str)
                        .map(|kind| format!("{kind}"))
                };
                ui.horizontal(|ui| {
                    let number = format!("{:02}", index + 1);
                    ui.label(
                        RichText::new(number)
                            .size(8.0)
                            .color(with_alpha(Palette::ACCENT, 0.75)),
                    );
                    ui.label(
                        RichText::new(summary.unwrap_or_else(|| format!("{noun} {}", index + 1)))
                            .size(9.5)
                            .color(Palette::TEXT),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if mini_button(ui, MiniIcon::Close, true).clicked() {
                            remove = Some(index);
                        }
                        if mini_button(ui, MiniIcon::Down, index + 1 < length).clicked()
                            && index + 1 < length
                        {
                            swap = Some((index, index + 1));
                        }
                        if mini_button(ui, MiniIcon::Up, index > 0).clicked() && index > 0 {
                            swap = Some((index, index - 1));
                        }
                    });
                });
                value_editor(editor, ui, item, &path, 0);
            });
         
         
        let id = Id::new(("macro_step", &path));
        let hot = ui.ctx().animate_bool_with_time(
            id.with("hover"),
            ui.rect_contains_pointer(step.response.rect),
            0.14,
        );
        if hot > 0.01 {
            let rect = step.response.rect;
            ui.painter().rect_stroke(
                rect,
                Rounding::same(8.0),
                Stroke::new(1.0_f32, with_alpha(Palette::ACCENT, 0.30 * hot)),
            );
            ui.painter().rect_filled(
                Rect::from_center_size(
                    pos2(rect.left() + 1.5, rect.center().y),
                    vec2(2.5, 16.0 + 10.0 * hot),
                ),
                Rounding::same(1.5),
                with_alpha(Palette::ACCENT, 0.55 * hot),
            );
        }
        if *item != before {
            changed = true;
        }
    }

    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt(format!("add_{field}"))
            .selected_text(format!("Add {noun}"))
            .width(116.0)
            .show_ui(ui, |ui| {
                for kind in types {
                    if ui.selectable_label(false, *kind).clicked() {
                        add = Some(*kind);
                    }
                }
            });
        ui.label(
            RichText::new("Steps are saved with the macro")
                .size(8.0)
                .color(with_alpha(Palette::TEXT_DIM, 0.5)),
        );
    });

    if let Some(index) = remove {
        array.remove(index);
        changed = true;
    }
    if let Some((a, b)) = swap {
        array.swap(a, b);
        changed = true;
    }
    if let Some(kind) = add {
        let item = default_of(kind, noun);
        array.push(item);
        changed = true;
    }
    if changed {
        if let Some(object) = editor.doc.as_object_mut() {
            object.insert(field.to_string(), Json::Array(array));
        }
    }
}

 
 
 
fn value_editor(editor: &mut Editor, ui: &mut Ui, value: &mut Json, path: &str, depth: usize) {
    if depth > 10 {
        ui.label(
            RichText::new("nested too deeply — edit in JSON")
                .size(8.5)
                .color(Palette::ERR),
        );
        return;
    }
    match value {
        Json::Object(object) => {
            let keys: Vec<String> = object.keys().cloned().collect();
            for key in keys {
                let child_path = format!("{path}.{key}");
                if key == "type" {
                     
                     
                    let current = object
                        .get("type")
                        .and_then(Json::as_str)
                        .unwrap_or("")
                        .to_string();
                    let choices = type_choices(path);
                    let mut chosen = None;
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("type")
                                .size(8.5)
                                .color(with_alpha(Palette::TEXT_DIM, 0.7)),
                        );
                        egui::ComboBox::from_id_salt(format!("type_{child_path}"))
                            .selected_text(&current)
                            .width(150.0)
                            .show_ui(ui, |ui| {
                                for choice in &choices {
                                    if ui.selectable_label(current == *choice, *choice).clicked() {
                                        chosen = Some(*choice);
                                    }
                                }
                            });
                    });
                    if let Some(choice) = chosen {
                        match typed_default(choice, path)
                            .and_then(|value| value.as_object().cloned())
                        {
                            Some(replacement) => *object = replacement,
                            None => {
                                object.insert("type".into(), Json::String(choice.into()));
                            }
                        }
                    }
                    continue;
                }
                let Some(original) = object.get(&key).cloned() else {
                    continue;
                };
                let mut child = original.clone();
                match &mut child {
                    Json::Bool(flag) => {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(human(&key))
                                    .size(8.5)
                                    .color(with_alpha(Palette::TEXT_DIM, 0.8)),
                            );
                            ui.checkbox(flag, "");
                        });
                    }
                    Json::Number(_) => {
                        let mut buffer = editor.buffer(&child_path, original.to_string());
                        ui.horizontal(|ui| {
                            ui.add_sized(
                                [90.0, 18.0],
                                egui::Label::new(
                                    RichText::new(human(&key))
                                        .size(8.5)
                                        .color(with_alpha(Palette::TEXT_DIM, 0.8)),
                                ),
                            );
                            let response = ui.add_sized(
                                [74.0, 20.0],
                                TextEdit::singleline(&mut buffer)
                                    .margin(Margin::symmetric(4.0, 2.0)),
                            );
                            if response.changed() {
                                if let Ok(number) = buffer.trim().parse::<i64>() {
                                    child = Json::from(number);
                                } else if let Ok(number) = buffer.trim().parse::<f64>() {
                                    if let Some(number) = serde_json::Number::from_f64(number) {
                                        child = Json::Number(number);
                                    }
                                }
                            }
                        });
                        editor.put_buffer(&child_path, buffer);
                    }
                    Json::String(text) => {
                        let mut buffer = editor.buffer(&child_path, text.clone());
                        ui.horizontal(|ui| {
                            ui.add_sized(
                                [66.0, 18.0],
                                egui::Label::new(
                                    RichText::new(human(&key))
                                        .size(8.5)
                                        .color(with_alpha(Palette::TEXT_DIM, 0.8)),
                                ),
                            );
                            let response = ui.add_sized(
                                [(ui.available_width() - 4.0).max(60.0), 20.0],
                                TextEdit::singleline(&mut buffer)
                                    .margin(Margin::symmetric(4.0, 2.0)),
                            );
                            if response.changed() {
                                *text = buffer.clone();
                            }
                        });
                        editor.put_buffer(&child_path, buffer);
                    }
                    other => {
                        ui.label(
                            RichText::new(human(&key))
                                .size(8.5)
                                .color(with_alpha(Palette::ACCENT, 0.75)),
                        );
                        if depth < 6 {
                            value_editor(editor, ui, other, &child_path, depth + 1);
                        } else {
                            let mut text = other.to_string();
                            let response = ui.add_sized(
                                [ui.available_width(), 20.0],
                                TextEdit::singleline(&mut text),
                            );
                            if response.changed() {
                                if let Ok(parsed) = serde_json::from_str::<Json>(&text) {
                                    *other = parsed;
                                }
                            }
                        }
                    }
                }
                if child != original {
                    object.insert(key, child);
                }
            }
        }
        Json::Array(items) => {
            let length = items.len();
            let mut remove = None;
            let mut add = None;
            for (index, item) in items.iter_mut().enumerate() {
                let child_path = format!("{path}[{index}]");
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("item {}", index + 1))
                            .size(8.0)
                            .color(with_alpha(Palette::TEXT_DIM, 0.5)),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if mini_button(ui, MiniIcon::Close, true).clicked() {
                            remove = Some(index);
                        }
                    });
                });
                value_editor(editor, ui, item, &child_path, depth + 1);
            }
            if remove.is_none() {
                ui.horizontal(|ui| {
                    if chip_button(ui, "ADD ITEM", 72.0, true).clicked() {
                        add = Some(Json::Object(Default::default()));
                    }
                });
            }
            if let Some(index) = remove {
                items.remove(index);
            }
            if let Some(item) = add {
                items.push(item);
            }
            if length == 0 {
                ui.label(
                    RichText::new("empty")
                        .size(8.0)
                        .color(with_alpha(Palette::TEXT_DIM, 0.5)),
                );
            }
        }
        Json::Bool(flag) => {
            ui.checkbox(flag, "");
        }
        Json::Null => {
            ui.label(
                RichText::new("null")
                    .size(8.5)
                    .color(with_alpha(Palette::TEXT_DIM, 0.6)),
            );
        }
        other => {
            let mut text = other.to_string();
            if ui.text_edit_singleline(&mut text).changed() {
                if let Ok(parsed) = serde_json::from_str::<Json>(&text) {
                    *other = parsed;
                }
            }
        }
    }
}

fn raw_tab(editor: &mut Editor, ui: &mut Ui) {
    ui.horizontal(|ui| {
        if chip_button(ui, "APPLY JSON", 86.0, false).clicked() {
            editor.apply_raw();
        }
        ui.label(
            RichText::new("invalid text is preserved, never saved")
                .size(8.5)
                .color(with_alpha(Palette::TEXT_DIM, 0.6)),
        );
    });
    let height = (ui.available_height() - 16.0).max(120.0);
    let response = ui.add_sized(
        [ui.available_width(), height],
        TextEdit::multiline(&mut editor.raw)
            .code_editor()
            .desired_width(f32::INFINITY)
            .margin(Margin::symmetric(8.0, 6.0)),
    );
    if response.changed() {
        editor.raw_error = None;
    }
}

 
 
fn section(ui: &mut Ui, title: &str, hint: &str) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 15.0), Sense::hover());
    let painter = ui.painter();
    let y = rect.center().y;
    painter.add(Shape::convex_polygon(
        vec![
            pos2(rect.left() + 3.0, y - 2.5),
            pos2(rect.left() + 5.5, y),
            pos2(rect.left() + 3.0, y + 2.5),
            pos2(rect.left() + 0.5, y),
        ],
        with_alpha(Palette::ACCENT, 0.9),
        Stroke::new(0.0_f32, Color32::TRANSPARENT),
    ));
    let title_w = tracked(
        painter,
        title,
        7.5,
        with_alpha(Palette::TEXT, 0.88),
        Align::Min,
        rect.left() + 12.0,
        y,
    );
    let hint_x = rect.left() + 12.0 + title_w + 8.0;
    let galley = elided(
        ui,
        hint,
        8.0,
        with_alpha(Palette::TEXT_DIM, 0.55),
        (rect.right() - hint_x - 8.0).max(0.0),
    );
    painter.galley(
        pos2(hint_x, y - galley.size().y * 0.5),
        galley.clone(),
        Palette::TEXT_DIM,
    );
    let line_x = hint_x + galley.size().x + 8.0;
    if rect.right() - line_x > 8.0 {
        painter.line_segment(
            [pos2(line_x, y), pos2(rect.right(), y)],
            Stroke::new(1.0_f32, with_alpha(Palette::TEXT_DIM, 0.10)),
        );
    }
}

 
#[derive(Clone, Copy)]
enum MiniIcon {
    Up,
    Down,
    Close,
}

fn mini_button(ui: &mut Ui, icon: MiniIcon, enabled: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(17.0), Sense::click());
    let hover = ui.ctx().animate_bool_with_time(
        response.id.with("hover"),
        response.hovered() && enabled,
        0.12,
    );
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        Rounding::same(5.0),
        with_alpha(Palette::TEXT_DIM, 0.06 + 0.12 * hover),
    );
    let ink = with_alpha(
        lerp_color(Palette::TEXT_DIM, Palette::TEXT, hover),
        if enabled { 0.61 + 0.28 * hover } else { 0.22 },
    );
    let stroke = Stroke::new(1.2_f32, ink);
    let c = rect.center();
    match icon {
        MiniIcon::Up => {
            painter.line_segment([c + vec2(-3.0, 1.6), c + vec2(0.0, -1.6)], stroke);
            painter.line_segment([c + vec2(3.0, 1.6), c + vec2(0.0, -1.6)], stroke);
            painter.line_segment([c + vec2(-2.6, 4.0), c + vec2(2.6, 4.0)], stroke);
        }
        MiniIcon::Down => {
            painter.line_segment([c + vec2(-3.0, -1.6), c + vec2(0.0, 1.6)], stroke);
            painter.line_segment([c + vec2(3.0, -1.6), c + vec2(0.0, 1.6)], stroke);
            painter.line_segment([c + vec2(-2.6, -4.0), c + vec2(2.6, -4.0)], stroke);
        }
        MiniIcon::Close => {
            painter.line_segment([c + vec2(-2.6, -2.6), c + vec2(2.6, 2.6)], stroke);
            painter.line_segment([c + vec2(2.6, -2.6), c + vec2(-2.6, 2.6)], stroke);
        }
    }
    if response.hovered() && enabled {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response
}

 
fn chip_button(ui: &mut Ui, label: &str, width: f32, plus: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, 20.0), Sense::click());
    let hover =
        ui.ctx()
            .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.14);
    let press = ui.ctx().animate_bool_with_time(
        response.id.with("press"),
        response.is_pointer_button_down_on(),
        0.08,
    );
    let body = rect.translate(vec2(0.0, press));
    let painter = ui.painter();
    painter.rect_filled(
        body,
        Rounding::same(6.0),
        with_alpha(Palette::CARD, 0.50 + 0.20 * hover),
    );
    painter.rect_stroke(
        body,
        Rounding::same(6.0),
        Stroke::new(
            1.0_f32,
            with_alpha(
                lerp_color(Palette::TEXT_DIM, Palette::ACCENT, hover),
                0.14 + 0.26 * hover,
            ),
        ),
    );
    let ink = with_alpha(Palette::TEXT, 0.62 + 0.30 * hover);
    let mut text_x = body.left() + 9.0;
    if plus {
        let c = pos2(body.left() + 9.0, body.center().y);
        painter.line_segment(
            [c + vec2(-2.6, 0.0), c + vec2(2.6, 0.0)],
            Stroke::new(1.2_f32, ink),
        );
        painter.line_segment(
            [c + vec2(0.0, -2.6), c + vec2(0.0, 2.6)],
            Stroke::new(1.2_f32, ink),
        );
        text_x = c.x + 8.0;
    }
    tracked(
        painter,
        label,
        6.5,
        ink,
        Align::Min,
        text_x,
        body.center().y,
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response
}

fn human(field: &str) -> String {
    field
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

 

fn action_type_names() -> Vec<&'static str> {
    infered_macros::ActionKind::ALL
        .iter()
        .map(|kind| kind.serde_name())
        .collect()
}

fn trigger_types() -> Vec<&'static str> {
    vec![
        "manual",
        "key_press",
        "key_release",
        "mouse_press",
        "mouse_release",
        "interval",
        "after_delay",
        "condition",
        "world_loaded",
        "world_unloaded",
        "player_joined",
        "player_left",
        "player_entered_range",
        "player_left_range",
        "player_distance_changed",
        "health_below",
        "hunger_below",
        "slot_changed",
        "inventory_changed",
        "equipment_changed",
        "level_changed",
        "effect_gained",
        "effect_lost",
        "screen_opened",
        "screen_closed",
        "started_moving",
        "stopped_moving",
        "entered_water",
        "left_water",
        "started_falling",
        "landed",
    ]
}

fn type_choices(path: &str) -> Vec<&'static str> {
    if path.contains("triggers") {
        trigger_types()
    } else if path.contains("conditions") || path.contains(".when") {
        vec!["compare", "all", "any", "not"]
    } else {
        action_type_names()
    }
}

fn default_of(kind: &str, noun: &str) -> Json {
    match noun {
        "action" => typed_default(kind, "actions").unwrap_or(Json::Object(Default::default())),
        "trigger" => trigger_default(kind),
        "condition" => condition_default(kind),
        _ => Json::Object(Default::default()),
    }
}

fn typed_default(kind: &str, context: &str) -> Option<Json> {
    if context.contains("conditions") || context == "condition" {
        return Some(condition_default(kind));
    }
    if context.contains("triggers") {
        return Some(trigger_default(kind));
    }
    infered_macros::ActionKind::ALL
        .iter()
        .find(|candidate| candidate.serde_name() == kind)
        .and_then(|candidate| serde_json::to_value(candidate.default_action()).ok())
}

fn trigger_default(kind: &str) -> Json {
    match kind {
        "manual" => serde_json::json!({ "type": "manual" }),
        "key_press" => serde_json::json!({ "type": "key_press", "key": "F8" }),
        "key_release" => serde_json::json!({ "type": "key_release", "key": "F8" }),
        "mouse_press" => serde_json::json!({ "type": "mouse_press", "button": 1 }),
        "mouse_release" => serde_json::json!({ "type": "mouse_release", "button": 1 }),
        "interval" => {
            serde_json::json!({ "type": "interval", "every_ms": 5000, "start_delay_ms": 0 })
        }
        "after_delay" => serde_json::json!({ "type": "after_delay", "delay_ms": 2000 }),
        "condition" => serde_json::json!({
            "type": "condition",
            "when": condition_default("compare"),
            "mode": "once",
            "every_ms": 0
        }),
        "world_loaded" | "world_unloaded" | "slot_changed" | "inventory_changed"
        | "equipment_changed" | "level_changed" | "started_moving" | "stopped_moving"
        | "entered_water" | "left_water" | "started_falling" | "landed" => {
            serde_json::json!({ "type": kind })
        }
        "player_joined" | "player_left" => {
            serde_json::json!({ "type": kind, "name": Json::Null })
        }
        "player_entered_range" | "player_left_range" => {
            serde_json::json!({ "type": kind, "range": 8.0, "name": Json::Null })
        }
        "player_distance_changed" => {
            serde_json::json!({ "type": kind, "by": 4.0, "name": Json::Null })
        }
        "health_below" | "hunger_below" => serde_json::json!({ "type": kind, "value": 10.0 }),
        "effect_gained" | "effect_lost" => {
            serde_json::json!({ "type": kind, "effect": Json::Null })
        }
        "screen_opened" | "screen_closed" => {
            serde_json::json!({ "type": kind, "screen": "inventory" })
        }
        _ => serde_json::json!({ "type": kind }),
    }
}

fn condition_default(kind: &str) -> Json {
    match kind {
        "all" | "any" => serde_json::json!({
            "type": kind,
            "conditions": [condition_default("compare")]
        }),
        "not" => serde_json::json!({
            "type": "not",
            "condition": condition_default("compare")
        }),
        _ => serde_json::json!({
            "type": "compare",
            "left": { "type": "state", "path": "health" },
            "op": "lt",
            "right": { "type": "literal", "value": 10.0 }
        }),
    }
}

 

fn export_macro(gui: &mut InjectorGui, file_name: &str) {
    let default = file_name.to_string();
    let Some(destination) = pick_save_file(&default) else {
        return;
    };
    match export(&MacrosPage::directory(), file_name, &destination) {
        Ok(path) => {
            gui.macros_page.notice = Some((format!("Exported to {}.", path.display()), true))
        }
        Err(error) => gui.macros_page.notice = Some((error, false)),
    }
}

fn open_folder(dir: &Path) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer").arg(dir).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(dir).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(dir).spawn();
    }
}

 
 
#[cfg(windows)]
fn pick_save_file(default_name: &str) -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::UI::Controls::Dialogs::{
        GetSaveFileNameW, OFN_NOCHANGEDIR, OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST, OPENFILENAMEW,
    };

    let filter: Vec<u16> = "Macro files (*.macro)\0*.macro\0All files (*.*)\0*.*\0\0"
        .encode_utf16()
        .collect();
    let mut file = vec![0u16; 1024];
    for (slot, unit) in file.iter_mut().zip(default_name.encode_utf16()) {
        *slot = unit;
    }
    let mut dialog: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    dialog.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    dialog.lpstrFilter = filter.as_ptr();
    dialog.nFilterIndex = 1;
    dialog.lpstrFile = file.as_mut_ptr();
    dialog.nMaxFile = file.len() as u32;
    dialog.Flags = OFN_OVERWRITEPROMPT | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;
    if unsafe { GetSaveFileNameW(&mut dialog) } == 0 {
        return None;
    }
    let len = file
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(file.len());
    let path = PathBuf::from(std::ffi::OsString::from_wide(&file[..len]));
    Some(if path.extension().is_none() {
        path.with_extension("macro")
    } else {
        path
    })
}

#[cfg(not(windows))]
fn pick_save_file(_default_name: &str) -> Option<PathBuf> {
    None
}

 
 
fn confirmation(gui: &mut InjectorGui, ui: &mut Ui, area: Rect, now: f64, appear: f32) {
    let Some(file) = gui.macros_page.confirm_delete.clone() else {
        return;
    };
    let name = gui
        .macros_page
        .entries
        .iter()
        .find(|entry| entry.file_name == file)
        .map(|entry| entry.name.clone())
        .unwrap_or_else(|| file.clone());

    let scale = 0.94 + 0.06 * appear;
    let card = Rect::from_center_size(area.center(), vec2(304.0, 132.0) * scale);
     
     
    let scrim = ui.interact(area, Id::new("macro_delete_scrim"), Sense::click());
    let mut painter = ui.painter().clone();
    painter.set_opacity(appear.clamp(0.0, 1.0));
    painter.rect_filled(
        area,
        Rounding::ZERO,
        Color32::from_black_alpha((150.0 * appear) as u8),
    );
    painter.rect_filled(
        card.translate(vec2(0.0, 4.0)).expand(2.0),
        Rounding::same(13.0),
        Color32::from_black_alpha(120),
    );
    painter.rect_filled(card, Rounding::same(12.0), Palette::BG);
    top_light_rounded(&painter, card, 12.0, 0.06);
    painter.rect_stroke(
        card,
        Rounding::same(12.0),
        Stroke::new(1.0_f32, with_alpha(Palette::ACCENT, 0.36)),
    );
    painter.line_segment(
        [
            pos2(card.left() + 14.0, card.top() + 1.0),
            pos2(card.right() - 14.0, card.top() + 1.0),
        ],
        Stroke::new(1.5_f32, with_alpha(Palette::ACCENT, 0.55)),
    );

     
    let well = Rect::from_center_size(
        pos2(card.left() + 16.0 + 17.0, card.top() + 17.0 + 17.0),
        Vec2::splat(34.0),
    );
    painter.rect_filled(well, Rounding::same(9.0), with_alpha(Palette::ERR, 0.10));
    painter.rect_stroke(
        well,
        Rounding::same(9.0),
        Stroke::new(1.0_f32, with_alpha(Palette::ERR, 0.34)),
    );
    let c = well.center();
    let ink = Stroke::new(1.3_f32, with_alpha(Palette::TEXT, 0.82));
    painter.line_segment([c + vec2(-5.5, -4.5), c + vec2(5.5, -4.5)], ink);
    painter.line_segment([c + vec2(-2.4, -6.5), c + vec2(2.4, -6.5)], ink);
    painter.rect_stroke(
        Rect::from_min_max(c + vec2(-4.4, -2.5), c + vec2(4.4, 6.0)),
        Rounding {
            nw: 1.5,
            ne: 1.5,
            sw: 2.5,
            se: 2.5,
        },
        ink,
    );
    for dx in [-1.6, 1.6] {
        painter.line_segment([c + vec2(dx, 0.4), c + vec2(dx, 3.8)], ink);
    }

    let text_x = well.right() + 12.0;
    painter.text(
        pos2(text_x, card.top() + 16.0),
        Align2::LEFT_TOP,
        "Delete this macro?",
        FontId::proportional(12.5),
        Palette::TEXT,
    );
    let detail = elided(
        ui,
        &name,
        9.5,
        with_alpha(Palette::TEXT_DIM, 0.85),
        card.right() - 14.0 - text_x,
    );
    painter.galley(pos2(text_x, card.top() + 37.0), detail, Palette::TEXT_DIM);
    painter.line_segment(
        [
            pos2(card.left() + 16.0, card.top() + 68.0),
            pos2(card.right() - 16.0, card.top() + 68.0),
        ],
        Stroke::new(1.0_f32, with_alpha(Palette::TEXT_DIM, 0.12)),
    );
    painter.text(
        pos2(card.left() + 16.0, card.top() + 78.0),
        Align2::LEFT_TOP,
        "This removes the .macro file from disk.",
        FontId::proportional(9.0),
        with_alpha(Palette::TEXT_DIM, 0.70),
    );

    let cancel = Rect::from_min_size(
        pos2(card.right() - 16.0 - 174.0, card.bottom() - 36.0),
        vec2(80.0, 24.0),
    );
    let delete = Rect::from_min_size(
        pos2(card.right() - 16.0 - 88.0, card.bottom() - 36.0),
        vec2(88.0, 24.0),
    );
    let cancel_response = ui.interact(cancel, Id::new("macro_delete_cancel"), Sense::click());
    let delete_response = ui.interact(delete, Id::new("macro_delete_ok"), Sense::click());
    let cancel_hover = ui.ctx().animate_bool_with_time(
        cancel_response.id.with("hover"),
        cancel_response.hovered(),
        0.14,
    );
    let delete_hover = ui.ctx().animate_bool_with_time(
        delete_response.id.with("hover"),
        delete_response.hovered(),
        0.14,
    );
    let delete_press = ui.ctx().animate_bool_with_time(
        delete_response.id.with("press"),
        delete_response.is_pointer_button_down_on(),
        0.08,
    );
    painter.rect_filled(
        cancel,
        Rounding::same(8.0),
        with_alpha(Palette::TEXT_DIM, 0.07 + 0.11 * cancel_hover),
    );
    painter.rect_stroke(
        cancel,
        Rounding::same(8.0),
        Stroke::new(
            1.0_f32,
            with_alpha(Palette::TEXT_DIM, 0.22 + 0.18 * cancel_hover),
        ),
    );
    tracked(
        &painter,
        "CANCEL",
        7.0,
        with_alpha(Palette::TEXT, 0.62 + 0.28 * cancel_hover),
        Align::Center,
        cancel.center().x,
        cancel.center().y,
    );
    glossy_pill(
        &painter,
        delete,
        PillStyle {
            label: "DELETE",
            icon: PillIcon::None,
            emphasis: 1.0,
        },
        delete_hover,
        delete_press,
        now,
    );

    if cancel_response.hovered() || delete_response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    if cancel_response.clicked() {
        gui.macros_page.confirm_delete = None;
    }
    if delete_response.clicked() {
        gui.macros_page.delete_confirmed(&file);
    }
    let outside = ui
        .ctx()
        .pointer_interact_pos()
        .is_none_or(|point| !card.contains(point));
    if scrim.clicked() && outside {
        gui.macros_page.confirm_delete = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Context, Pos2, RawInput};

     
    fn painted(mut paint: impl FnMut(&mut Ui)) -> (Vec<String>, Vec<Shape>) {
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
                    .show(ctx, |ui| paint(ui));
            },
        );
        let mut texts = Vec::new();
        let mut shapes = Vec::new();
        for clipped in output.shapes {
            match clipped.shape {
                Shape::Text(text) => texts.push(text.galley.text().to_owned()),
                other => shapes.push(other),
            }
        }
        (texts, shapes)
    }

    fn icon(ctx: &Context) -> TextureHandle {
        ctx.load_texture(
            "test_macros_icon",
            egui::ColorImage::from_rgba_unmultiplied([1, 1], &[255, 255, 255, 255]),
            Default::default(),
        )
    }

    fn row(name: &str, trigger: &str, enabled: bool, actions: usize) -> Entry {
        Entry {
            file_name: format!("{}.macro", name.to_lowercase().replace(' ', "_")),
            id: name.to_lowercase().replace(' ', "_"),
            name: name.to_string(),
            description: String::new(),
            triggers: vec![trigger.to_string()],
            enabled,
            modified: 1,
            action_count: actions,
            error: None,
            warning: None,
        }
    }

    #[test]
    fn the_header_counts_macros_and_offers_a_new_one() {
        let (texts, _) = painted(|ui| {
            let ctx = ui.ctx();
            let icon = icon(ctx);
            let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(670.0, 278.0));
            let _ = header(
                ui,
                area,
                &icon,
                HeaderCounts {
                    loaded: 3,
                    active: 2,
                    broken: 0,
                },
                5.0,
                10.0,
            );
        });
        let joined = texts.join("");
        for expected in ["MACROS", "3 MACROS  ·  2 ACTIVE", "NEW MACRO"] {
            assert!(
                joined.contains(expected),
                "missing {expected:?} in {joined:?}"
            );
        }
    }

    #[test]
    fn unreadable_files_replace_the_active_count() {
        let (texts, _) = painted(|ui| {
            let ctx = ui.ctx();
            let icon = icon(ctx);
            let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(670.0, 278.0));
            let _ = header(
                ui,
                area,
                &icon,
                HeaderCounts {
                    loaded: 2,
                    active: 1,
                    broken: 1,
                },
                5.0,
                10.0,
            );
        });
        assert!(
            texts.join("").contains("2 MACROS  ·  1 UNREADABLE"),
            "got {texts:?}"
        );
    }

    #[test]
    fn a_card_shows_its_name_trigger_and_actions() {
        let (texts, shapes) = painted(|ui| {
            let entry = row("Strafe Combo", "F8", true, 3);
            let _ = macro_card(ui, &entry, true, 0, 5.0, 0.0);
        });
        let joined = texts.join("");
        for expected in ["Strafe Combo", "F8", "3 actions"] {
            assert!(
                joined.contains(expected),
                "missing {expected:?} in {joined:?}"
            );
        }
        assert!(
            shapes.iter().any(|shape| matches!(shape, Shape::Circle(_))),
            "the card must paint its switch"
        );
        assert!(
            shapes
                .iter()
                .any(|shape| matches!(shape, Shape::Path(path) if path.closed && path.fill != Color32::TRANSPARENT)),
            "the play mark must be filled"
        );
    }

    #[test]
    fn the_empty_library_offers_a_new_macro() {
        let (texts, _) = painted(|ui| {
            let _ = empty_state(ui, true, 5.0, -10.0);
        });
        let joined = texts.join("");
        for expected in ["No macros yet", "NEW MACRO", "drop a .macro"] {
            assert!(
                joined.contains(expected),
                "missing {expected:?} in {joined:?}"
            );
        }
    }

    #[test]
    fn the_delete_dialog_names_the_macro_and_offers_both_actions() {
        let mut gui = InjectorGui::stub_gui();
        gui.macros_page.stub_confirm("strafe_combo.macro");
        gui.macros_page.entries = vec![row("Strafe Combo", "F8", true, 4)];
        let (texts, _) = painted(|ui| {
            let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(670.0, 278.0));
            confirmation(&mut gui, ui, area, 5.0, 1.0);
        });
        let joined = texts.join("");
        for expected in [
            "Delete this macro?",
            "Strafe Combo",
            "removes the .macro file",
            "CANCEL",
            "DELETE",
        ] {
            assert!(
                joined.contains(expected),
                "missing {expected:?} in {joined:?}"
            );
        }
    }

    #[test]
    fn the_preview_editor_document_is_a_valid_macro() {
        let mut page = MacrosPage::default();
        page.stub_editor();
        let editor = page.editor.expect("stub editor");
        assert!(
            serde_json::from_value::<Macro>(editor.doc).is_ok(),
            "the preview document must parse as a macro"
        );
    }

    #[test]
    fn defaults_for_every_action_parse() {
        for kind in infered_macros::ActionKind::ALL {
            let json = default_of(kind.serde_name(), "action");
            let parsed = serde_json::from_value::<infered_macros::Action>(json);
            assert!(parsed.is_ok(), "{} has a broken default", kind.serde_name());
        }
    }

    #[test]
    fn defaults_for_every_trigger_parse() {
        for kind in trigger_types() {
            let json = trigger_default(kind);
            let parsed = serde_json::from_value::<infered_macros::Trigger>(json);
            assert!(parsed.is_ok(), "{kind} has a broken default");
        }
    }

    #[test]
    fn defaults_for_every_condition_parse() {
        for kind in ["compare", "all", "any", "not"] {
            let json = condition_default(kind);
            let parsed = serde_json::from_value::<infered_macros::Condition>(json);
            assert!(parsed.is_ok(), "{kind} has a broken default");
        }
    }

    #[test]
    fn human_labels_are_readable() {
        assert_eq!(human("duration_ms"), "Duration Ms");
        assert_eq!(human("in_water"), "In Water");
    }
}
