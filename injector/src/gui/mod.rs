 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

mod addons;
mod config;
mod friends;
mod macros;
mod menu;
mod theme;
mod widgets;

use std::collections::{BTreeSet, HashMap};
use std::time::Duration;

use eframe::{Frame, NativeOptions};
use egui::{
    pos2, Align2, Area, Color32, Context, FontId, Id, Order, Rect, RichText, Rounding, Sense,
    Stroke, TextureHandle, Vec2, ViewportBuilder, ViewportCommand,
};

use crate::app::{InjectionStatus, InjectorApp};
use crate::friends::FriendView;
use crate::modules::ModuleView;

 
mod assets {
     
    pub const ICON_PNG: &[u8] = include_bytes!("../../assets/icon.png");
     
    pub const TEXTLOGO_PNG: &[u8] = include_bytes!("../../assets/textlogo.png");
     
    pub const SIDEBAR_PNG: &[u8] = include_bytes!("../../../logo's/sidebar.png");
     
    pub const MINIMIZE_PNG: &[u8] = include_bytes!("../../../logo's/minimize.png");
     
    pub const CLOSE_PNG: &[u8] = include_bytes!("../../../logo's/close.png");
     
    pub const BACK_PNG: &[u8] = include_bytes!("../../../logo's/back.png");
     
    pub const ADDONS_PNG: &[u8] = include_bytes!("../../../logo's/addons.png");
     
    pub const CONFIG_PNG: &[u8] = include_bytes!("../../../logo's/config.png");
     
    pub const FRIENDS_PNG: &[u8] = include_bytes!("../../../logo's/friends.png");
     
     
    pub const MACROS_PNG: &[u8] = include_bytes!("../../../logo's/settings.png");
}

 
const FRAME_TIME: Duration = Duration::from_millis(16);

 
const MIN_SIZE: [f32; 2] = [750.0, 450.0];

 
const TOAST_STAY_SECS: f64 = 3.0;
 
const TOAST_IN_SECS: f64 = 0.55;
 
const TOAST_OUT_SECS: f64 = 1.2;
 
const TOAST_SIZE: egui::Vec2 = egui::Vec2::new(300.0, 72.0);
const TOAST_MARGIN: f32 = 18.0;

 
 
 
const PAGE_SLIDE_SECS: f64 = 0.30;
const PAGE_SLIDE_PX: f32 = 26.0;

 
struct Toast {
     
    title: &'static str,
     
    detail: String,
    success: bool,
     
    shown_at: f64,
     
    dismissing: bool,
    dismissed_at: f64,
}

 
pub fn run() -> Result<(), eframe::Error> {
    ensure_dpi_aware();
    let native_options = NativeOptions {
         
         
        persist_window: false,
        viewport: ViewportBuilder::default()
            .with_title("Infered")
            .with_icon(window_icon())
            .with_inner_size(MIN_SIZE)
            .with_min_inner_size(MIN_SIZE)
            .with_max_inner_size(MIN_SIZE)
             
             
            .with_decorations(false)
            .with_resizable(false)
            .with_fullscreen(false)
            .with_maximized(false),
        ..Default::default()
    };
    eframe::run_native(
        "Infered",
        native_options,
        Box::new(|cc| {
            theme::apply(&cc.egui_ctx);
             
             
            cc.egui_ctx.set_pixels_per_point(1.0);
            let textlogo = texture(&cc.egui_ctx, "textlogo", assets::TEXTLOGO_PNG, 640);
            let sidebar_icon = texture(&cc.egui_ctx, "sidebar", assets::SIDEBAR_PNG, 64);
            let minimize_icon = texture(&cc.egui_ctx, "minimize", assets::MINIMIZE_PNG, 64);
            let close_icon = texture(&cc.egui_ctx, "close", assets::CLOSE_PNG, 64);
            let back_icon = texture(&cc.egui_ctx, "back", assets::BACK_PNG, 64);
            let addons_icon = texture(&cc.egui_ctx, "addons", assets::ADDONS_PNG, 64);
            let config_icon = texture(&cc.egui_ctx, "config", assets::CONFIG_PNG, 64);
            let friends_icon = texture(&cc.egui_ctx, "friends", assets::FRIENDS_PNG, 64);
            let macros_icon = texture(&cc.egui_ctx, "macros", assets::MACROS_PNG, 64);
            let mut app = InjectorApp::new();
            app.scan();
            let gui = InjectorGui {
                app,
                textlogo,
                sidebar_icon,
                minimize_icon,
                close_icon,
                back_icon,
                addons_icon,
                config_icon,
                friends_icon,
                macros_icon,
                macros_page: macros::MacrosPage::default(),
                addons_menu_open: false,
                process_menu_open: false,
                drawer_page: DrawerPage::Menu,
                page_entered_at: 0.0,
                page_direction: 1.0,
                addons_opened_at: 0.0,
                last_addon_refresh: 0.0,
                addon_notice: None,
                config_name: String::new(),
                config_notice: None,
                config_modules: Vec::new(),
                config_search: String::new(),
                config_expanded: BTreeSet::new(),
                config_buffers: HashMap::new(),
                config_opened_at: 0.0,
                friends_opened_at: 0.0,
                friends_list: Vec::new(),
                friend_name: String::new(),
                friend_notice: None,
                friends_expanded: BTreeSet::new(),
                toast: None,
                status_key: None,
            };
            Ok(Box::new(gui))
        }),
    )
}

#[cfg(windows)]
fn ensure_dpi_aware() {
    use windows_sys::Win32::UI::HiDpi::{
        SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE,
        DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    unsafe {
         
        if SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) == 0 {
            SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE);
        }
    }
}

#[cfg(not(windows))]
fn ensure_dpi_aware() {}

 
 
 
 
#[cfg(windows)]
fn apply_rounded_region() {
    use windows_sys::Win32::Foundation::{HWND, TRUE};
    use windows_sys::Win32::Graphics::Gdi::{CreateRoundRectRgn, SetWindowRgn};
    use windows_sys::Win32::System::Threading::GetCurrentProcessId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId,
    };

    struct Search {
        pid: u32,
        title: String,
        found: Option<HWND>,
    }

     
     
    unsafe extern "system" fn find_hwnd(hwnd: HWND, lparam: isize) -> i32 {
        let search = unsafe { &mut *(lparam as *mut Search) };
        let mut buf = [0u16; 256];
        let len = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
        if len <= 0 {
            return 1;  
        }
        let text = String::from_utf16_lossy(&buf[..len as usize]);
        if text == search.title {
            let mut pid: u32 = 0;
            unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
            if pid == search.pid {
                search.found = Some(hwnd);
                return 0;  
            }
        }
        1  
    }

    let mut search = Search {
        pid: unsafe { GetCurrentProcessId() },
        title: "Infered".to_string(),
        found: None,
    };
    unsafe { EnumWindows(Some(find_hwnd), &mut search as *mut Search as isize) };

    if let Some(hwnd) = search.found {
        unsafe {
            let mut rect = std::mem::zeroed();
            GetWindowRect(hwnd, &mut rect);
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;
             
            let rgn = CreateRoundRectRgn(0, 0, width, height, 28, 28);
            if rgn != 0 {
                SetWindowRgn(hwnd, rgn, TRUE);
            }
        }
    }
}

#[cfg(not(windows))]
fn apply_rounded_region() {}

fn window_icon() -> egui::IconData {
    let img = image::load_from_memory(assets::ICON_PNG)
        .expect("embedded icon.png is a valid PNG")
        .to_rgba8();
    let icon = image::imageops::resize(&img, 64, 64, image::imageops::FilterType::Lanczos3);
    egui::IconData {
        width: 64,
        height: 64,
        rgba: icon.into_raw(),
    }
}

 
fn texture(ctx: &Context, name: &str, png: &[u8], max_width: u32) -> TextureHandle {
    let img = image::load_from_memory(png)
        .expect("embedded PNG is a valid image")
        .to_rgba8();
    let img = if img.width() > max_width {
        let height =
            ((img.height() as f32 * max_width as f32 / img.width() as f32).round() as u32).max(1);
        image::imageops::resize(
            &img,
            max_width,
            height,
            image::imageops::FilterType::Lanczos3,
        )
    } else {
        img
    };
    let size = [img.width() as usize, img.height() as usize];
    ctx.load_texture(
        name,
        egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()),
        egui::TextureOptions::LINEAR,
    )
}

 
fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

 
fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t.powi(3)
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

 
 
 
pub(super) fn page_slide_offset(direction: f32, elapsed: f64) -> f32 {
    let t = (elapsed / PAGE_SLIDE_SECS).clamp(0.0, 1.0) as f32;
    direction * PAGE_SLIDE_PX * (1.0 - ease_out_cubic(t))
}

 
 
#[derive(Clone, Copy, PartialEq, Eq)]
enum DrawerPage {
    Menu,
    Addons,
    Config,
    Friends,
    Macros,
}

struct InjectorGui {
    app: InjectorApp,
    textlogo: TextureHandle,
    sidebar_icon: TextureHandle,
    minimize_icon: TextureHandle,
    close_icon: TextureHandle,
    back_icon: TextureHandle,
    addons_icon: TextureHandle,
    config_icon: TextureHandle,
    friends_icon: TextureHandle,
    macros_icon: TextureHandle,
     
    macros_page: macros::MacrosPage,
    addons_menu_open: bool,
     
    process_menu_open: bool,
    drawer_page: DrawerPage,
     
     
    page_entered_at: f64,
    page_direction: f32,
     
     
    addons_opened_at: f64,
    last_addon_refresh: f64,
     
    addon_notice: Option<(String, bool)>,
     
    config_name: String,
     
    config_notice: Option<(String, bool)>,
     
    config_modules: Vec<ModuleView>,
     
    config_search: String,
     
    config_expanded: BTreeSet<String>,
     
     
     
    config_buffers: HashMap<String, String>,
     
    config_opened_at: f64,
     
    friends_opened_at: f64,
     
    friends_list: Vec<FriendView>,
     
    friend_name: String,
     
    friend_notice: Option<(String, bool)>,
     
    friends_expanded: BTreeSet<String>,
     
    toast: Option<Toast>,
     
    status_key: Option<(std::mem::Discriminant<InjectionStatus>, String)>,
}

impl InjectorGui {
     
     
     
     
     
     
    fn enforce_min_size(&self, ctx: &Context) {
        ctx.send_viewport_cmd(ViewportCommand::Resizable(false));
        ctx.send_viewport_cmd(ViewportCommand::MinInnerSize(egui::vec2(
            MIN_SIZE[0],
            MIN_SIZE[1],
        )));
        ctx.send_viewport_cmd(ViewportCommand::MaxInnerSize(egui::vec2(
            MIN_SIZE[0],
            MIN_SIZE[1],
        )));
        let screen = ctx.input(|i| i.screen_rect);
        let w = screen.width();
        let h = screen.height();
        if (w - MIN_SIZE[0]).abs() > 0.5 || (h - MIN_SIZE[1]).abs() > 0.5 {
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(egui::vec2(
                MIN_SIZE[0],
                MIN_SIZE[1],
            )));
        }
    }

     
     
    fn update_toasts(&mut self, now: f64) {
        let status = self.app.status().clone();
        let key = (std::mem::discriminant(&status), status.message());
        if self.status_key.as_ref() == Some(&key) {
            return;
        }
        self.status_key = Some(key);
        match &status {
            InjectionStatus::Done(pid) => {
                self.toast = Some(Toast {
                    title: "successfully injected",
                    detail: format!("Injected into process {pid}."),
                    success: true,
                    shown_at: now,
                    dismissing: false,
                    dismissed_at: 0.0,
                });
            }
            InjectionStatus::Failed(reason) => {
                self.toast = Some(Toast {
                    title: "injection failed",
                    detail: reason.clone(),
                    success: false,
                    shown_at: now,
                    dismissing: false,
                    dismissed_at: 0.0,
                });
            }
            _ => {}
        }
    }

     
     
    fn toast_anim(toast: &mut Toast, now: f64, target_x: f32, off_x: f32) -> (f32, bool) {
         
        if !toast.dismissing && now - toast.shown_at > TOAST_STAY_SECS {
            toast.dismissing = true;
            toast.dismissed_at = now;
        }
        if toast.dismissing {
            let t = ((now - toast.dismissed_at) / TOAST_OUT_SECS).min(1.0);
            if t >= 1.0 {
                return (off_x, true);
            }
            let p = ease_in_out_cubic(t as f32);
            (off_x + (target_x - off_x) * p, false)
        } else {
            let t = ((now - toast.shown_at) / TOAST_IN_SECS).min(1.0);
            let p = ease_out_cubic(t as f32);
            (off_x + (target_x - off_x) * p, false)
        }
    }
}

 
fn pick_addon_file() -> Option<std::path::PathBuf> {
    pick_file("Infered addon (*.inf)\0*.inf\0All files (*.*)\0*.*\0\0")
}

 
fn pick_config_file() -> Option<std::path::PathBuf> {
    pick_file("Infered config (*.json)\0*.json\0All files (*.*)\0*.*\0\0")
}

 
 
 
#[cfg(windows)]
fn pick_file(filter: &str) -> Option<std::path::PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::UI::Controls::Dialogs::{
        GetOpenFileNameW, OFN_FILEMUSTEXIST, OFN_NOCHANGEDIR, OFN_PATHMUSTEXIST, OPENFILENAMEW,
    };

    let filter: Vec<u16> = filter.encode_utf16().collect();
    let mut file = vec![0u16; 1024];
    let mut dialog: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    dialog.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    dialog.lpstrFilter = filter.as_ptr();
    dialog.nFilterIndex = 1;
    dialog.lpstrFile = file.as_mut_ptr();
    dialog.nMaxFile = file.len() as u32;
    dialog.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;
    if unsafe { GetOpenFileNameW(&mut dialog) } == 0 {
        return None;
    }
    let len = file
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(file.len());
    Some(std::path::PathBuf::from(std::ffi::OsString::from_wide(
        &file[..len],
    )))
}

 
#[cfg(not(windows))]
fn pick_file(_filter: &str) -> Option<std::path::PathBuf> {
    None
}

impl eframe::App for InjectorGui {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
         
         
        if (ctx.pixels_per_point() - 1.0).abs() > f32::EPSILON {
            ctx.set_pixels_per_point(1.0);
        }

        self.enforce_min_size(ctx);
        let now = ctx.input(|i| i.time);

         
         
        if self.addons_menu_open
            && !ctx.wants_keyboard_input()
            && ctx.input(|input| input.key_pressed(egui::Key::Escape))
        {
            if self.drawer_page == DrawerPage::Menu {
                self.addons_menu_open = false;
            } else {
                self.open_page(DrawerPage::Menu, now, false);
            }
        }

        self.app.poll();
        self.update_toasts(now);

         
         
        #[cfg(windows)]
        apply_rounded_region();

         
        let mut addons_button_clicked = false;
        egui::TopBottomPanel::top("title_bar")
            .frame(theme::title_bar_frame())
            .show_separator_line(false)
            .show(ctx, |ui| {
                addons_button_clicked = widgets::title_bar(
                    ui,
                    &self.sidebar_icon,
                    &self.minimize_icon,
                    &self.close_icon,
                );
            });
        if addons_button_clicked {
            self.addons_menu_open = !self.addons_menu_open;
            self.drawer_page = DrawerPage::Menu;
            if self.addons_menu_open {
                 
                 
                 
                self.page_entered_at = now + 0.12;
                self.page_direction = 1.0;
            }
        }

        egui::CentralPanel::default()
            .frame(theme::central_frame())
            .show(ctx, |ui| {
                 
                 
                 
                 
                 
                let underline_center = ui.ctx().screen_rect().center().y;
                let content_top = theme::TITLE_BAR_HEIGHT + 14.0;
                let top = (underline_center - content_top - 138.0).max(0.0);
                ui.add_space(top);
                widgets::logo_banner(ui, &self.textlogo);
                ui.add_space(52.0);
                if widgets::inject_button(ui, &self.app).clicked() {
                    self.app.inject_selected();
                }
                ui.add_space(12.0);
                ui.vertical_centered(|ui| {
                    widgets::process_picker(ui, &mut self.app, &mut self.process_menu_open);
                });
            });

        self.paint_toast(ctx);
        self.paint_addons_drawer(ctx);

         
         
        ctx.request_repaint_after(FRAME_TIME);
    }
}

impl InjectorGui {
     
    fn paint_addons_drawer(&mut self, ctx: &Context) {
        let now = ctx.input(|input| input.time);
        let progress =
            ctx.animate_bool_with_time(Id::new("addons_drawer_open"), self.addons_menu_open, 0.90);
        if progress <= 0.001 {
            return;
        }
         
         
        if progress < 0.999 {
            ctx.request_repaint();
        }

        let screen = ctx.screen_rect();
         
         
        let width = (screen.width() - 48.0).max(420.0);
        let height = (screen.height() - 96.0).max(300.0);
        let target_x = (screen.width() - width) * 0.5;
        let target_y = (screen.height() - height) * 0.5;
         
         
        let eased = progress * progress * (3.0 - 2.0 * progress);
        let x = target_x;
        let y = target_y + 18.0 * (1.0 - eased);
         
         
        let scrim = ctx.layer_painter(egui::LayerId::new(
            Order::Middle,
            Id::new("addons_drawer_scrim"),
        ));
        scrim.rect_filled(
            screen,
            Rounding::ZERO,
            Color32::from_black_alpha((90.0 * eased) as u8),
        );
        Area::new(Id::new("addons_drawer"))
            .fixed_pos(pos2(x, y))
            .order(Order::Foreground)
            .show(ctx, |ui| {
                ui.set_opacity(eased);
                 
                 
                 
                for text_style in [
                    egui::TextStyle::Small,
                    egui::TextStyle::Body,
                    egui::TextStyle::Button,
                    egui::TextStyle::Heading,
                ] {
                    ui.style_mut().text_styles.insert(
                        text_style.clone(),
                        egui::FontId::proportional(match text_style {
                            egui::TextStyle::Small => 11.0,
                            egui::TextStyle::Heading => 18.0,
                            _ => 14.0,
                        }),
                    );
                }
                egui::Frame::none()
                    .fill(theme::Palette::BG)
                    .stroke(Stroke::new(
                        1.0_f32,
                        theme::with_alpha(theme::Palette::ACCENT, 0.20),
                    ))
                    .rounding(Rounding::same(16.0))
                    .inner_margin(egui::Margin::same(16.0))
                    .show(ui, |ui| {
                        ui.set_min_width(width - 32.0);
                        ui.set_max_width(width - 32.0);
                        ui.set_min_height(height - 32.0);
                        ui.horizontal(|ui| {
                            if self.drawer_page != DrawerPage::Menu {
                                let (rect, response) =
                                    ui.allocate_exact_size(egui::Vec2::splat(30.0), Sense::click());
                                widgets::paint_png_button(
                                    ui,
                                    &response,
                                    rect.center(),
                                    &self.back_icon,
                                );
                                if response.clicked() {
                                    self.open_page(DrawerPage::Menu, now, false);
                                }
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let (rect, response) = ui.allocate_exact_size(
                                        egui::Vec2::splat(30.0),
                                        Sense::click(),
                                    );
                                    widgets::paint_png_button(
                                        ui,
                                        &response,
                                        rect.center(),
                                        &self.close_icon,
                                    );
                                    if response.clicked() {
                                        self.addons_menu_open = false;
                                        self.drawer_page = DrawerPage::Menu;
                                    }
                                },
                            );
                        });
                        ui.add_space(14.0);

                         
                         
                         
                         
                         
                        let (page_rect, _) =
                            ui.allocate_exact_size(ui.available_size(), Sense::hover());
                        let elapsed = (now - self.page_entered_at).max(0.0);
                        let dx = page_slide_offset(self.page_direction, elapsed);
                        let mut page_ui = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(page_rect.translate(egui::vec2(dx, 0.0)))
                                .layout(egui::Layout::top_down(egui::Align::Min)),
                        );
                        page_ui.set_clip_rect(page_rect);
                        match self.drawer_page {
                            DrawerPage::Menu => self.paint_menu(&mut page_ui, now),
                            DrawerPage::Addons => self.paint_addon_page(&mut page_ui),
                            DrawerPage::Config => self.paint_config_page(&mut page_ui),
                            DrawerPage::Friends => self.paint_friends_page(&mut page_ui),
                            DrawerPage::Macros => self.paint_macros_page(&mut page_ui),
                        }
                    });
            });
    }

     
     
     
     
     
    fn open_page(&mut self, page: DrawerPage, now: f64, forward: bool) {
        if self.drawer_page == page {
            return;
        }
        self.drawer_page = page;
        self.page_entered_at = now;
        self.page_direction = if forward { 1.0 } else { -1.0 };
        match page {
            DrawerPage::Menu => {}
            DrawerPage::Addons => {
                self.addons_opened_at = now;
                self.app.refresh_addons();
            }
            DrawerPage::Config => {
                self.config_opened_at = now;
                self.config_modules = self.app.module_views();
                self.config_buffers.clear();
                self.app.refresh_configs();
            }
            DrawerPage::Friends => {
                self.friends_opened_at = now;
                self.friend_notice = None;
                self.friends_expanded.clear();
                self.friends_list = match self.app.friends() {
                    Ok(friends) => friends,
                    Err(error) => {
                        self.friend_notice = Some((error, false));
                        Vec::new()
                    }
                };
            }
            DrawerPage::Macros => {
                self.macros_page.open(now);
            }
        }
    }

     
    fn paint_menu(&mut self, ui: &mut egui::Ui, now: f64) {
        menu::page(self, ui, now);
    }

     
    fn save_config_from_name(&mut self) {
        let name = self.config_name.trim().to_string();
        self.config_notice = Some(match self.app.save_config(&name) {
            Ok(()) => (format!("Saved {name}."), true),
            Err(error) => (error, false),
        });
    }

     
    fn load_config_by_name(&mut self, name: &str) {
        self.config_notice = Some(match self.app.load_config(name) {
            Ok(()) => {
                self.refresh_config_views();
                (format!("Loaded {name}."), true)
            }
            Err(error) => (error, false),
        });
    }

     
     
    fn load_config_from_picker(&mut self) {
        let Some(path) = pick_config_file() else {
            return;
        };
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("config")
            .to_string();
        self.config_notice = Some(match self.app.load_config_file(&path) {
            Ok(()) => {
                self.refresh_config_views();
                (format!("Loaded {name}."), true)
            }
            Err(error) => (error, false),
        });
    }

     
     
    fn refresh_config_views(&mut self) {
        self.config_modules = self.app.module_views();
        self.config_buffers.clear();
        self.config_expanded.clear();
    }

    fn paint_config_page(&mut self, ui: &mut egui::Ui) {
        config::page(self, ui);
    }

    fn paint_friends_page(&mut self, ui: &mut egui::Ui) {
        friends::page(self, ui);
    }

    fn paint_macros_page(&mut self, ui: &mut egui::Ui) {
        macros::page(self, ui);
    }

     
     
    fn load_addon_from_picker(&mut self) {
        let Some(path) = pick_addon_file() else {
            return;
        };
        self.addon_notice = Some(match self.app.import_addon(&path) {
            Ok(name) => (format!("Loaded {name}."), true),
            Err(error) => (error, false),
        });
    }

     
     
    fn paint_addon_page(&mut self, ui: &mut egui::Ui) {
        let now = ui.input(|input| input.time);
        if now - self.last_addon_refresh >= 0.5 {
            self.app.refresh_addons();
            self.last_addon_refresh = now;
        }
        addons::page(self, ui, now);
    }

     
    fn paint_toast(&mut self, ctx: &Context) {
        let now = ctx.input(|i| i.time);
        let Some(toast) = &mut self.toast else {
            return;
        };
        let screen = ctx.screen_rect();
        let target_x = screen.right() - TOAST_MARGIN - TOAST_SIZE.x;
         
        let target_y = screen.top() + theme::TITLE_BAR_HEIGHT + TOAST_MARGIN;
         
        let off_x = screen.right() + 12.0;

        let (x, done) = Self::toast_anim(toast, now, target_x, off_x);
        if done {
            self.toast = None;
            return;
        }
         
        let remaining = if toast.dismissing {
            0.0
        } else {
            (1.0 - (now - toast.shown_at) / TOAST_STAY_SECS).clamp(0.0, 1.0) as f32
        };
        let rect = Rect::from_min_size(pos2(x, target_y), TOAST_SIZE);
        let status_color = if toast.success {
            theme::Palette::OK
        } else {
            theme::Palette::ERR
        };
        let title = toast.title;
        let detail = toast.detail.clone();
        let success = toast.success;

        let mut dismiss = false;
         
        Area::new(Id::new("toast"))
            .fixed_pos(rect.min)
            .order(Order::Tooltip)
            .show(ctx, |ui| {
                let (area, response) = ui.allocate_exact_size(TOAST_SIZE, Sense::click());
                if response.clicked() {
                    dismiss = true;
                }
                if response.hovered() {
                    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                }
                paint_toast_body(
                    ui,
                    area,
                    ToastLook {
                        color: status_color,
                        title,
                        detail: &detail,
                        success,
                        remaining,
                        hovered: response.hovered(),
                    },
                    now,
                );
            });
        if dismiss {
            if let Some(toast) = &mut self.toast {
                if !toast.dismissing {
                    toast.dismissing = true;
                    toast.dismissed_at = now;
                }
            }
        }
    }
}

 
struct ToastLook<'a> {
    color: Color32,
    title: &'a str,
    detail: &'a str,
    success: bool,
     
    remaining: f32,
    hovered: bool,
}

 
 
 
fn paint_toast_body(ui: &mut egui::Ui, rect: Rect, look: ToastLook<'_>, now: f64) {
    let painter = ui.painter();
    let rounding = Rounding::same(16.0);
    let color = look.color;

     
    painter.rect_filled(
        rect.translate(Vec2::new(0.0, 6.0)).expand(6.0),
        Rounding::same(22.0),
        Color32::from_black_alpha(90),
    );
     
    let breathe = 0.6 + 0.4 * theme::pulse(now, 2.8);
    painter.rect_filled(
        rect.expand(3.0),
        Rounding::same(19.0),
        theme::with_alpha(color, 0.10 * breathe),
    );

     
    painter.rect_filled(rect, rounding, theme::Palette::CARD);
    let border = if look.hovered { 0.6 } else { 0.28 };
    painter.rect_stroke(
        rect,
        rounding,
        Stroke::new(1.0_f32, theme::with_alpha(color, border)),
    );

     
    let chip = pos2(rect.left() + 36.0, rect.center().y - 2.0);
    painter.circle_filled(chip, 17.0, theme::with_alpha(color, 0.16));
    painter.circle_stroke(
        chip,
        17.0,
        Stroke::new(1.0_f32, theme::with_alpha(color, 0.7)),
    );
    painter.text(
        chip,
        Align2::CENTER_CENTER,
        if look.success { "✔" } else { "✖" },
        FontId::proportional(14.0),
        color,
    );

     
    let cross_color = if look.hovered {
        theme::Palette::TEXT
    } else {
        theme::Palette::TEXT_DIM
    };
    painter.text(
        pos2(rect.right() - 18.0, rect.top() + 18.0),
        Align2::CENTER_CENTER,
        "×",
        FontId::proportional(16.0),
        cross_color,
    );

     
    let text_x = rect.left() + 66.0;
    let text_width = rect.right() - 40.0 - text_x;
    painter.text(
        pos2(text_x, rect.top() + 22.0),
        Align2::LEFT_CENTER,
        look.title,
        FontId::proportional(14.5),
        theme::Palette::TEXT,
    );
    let galley = egui::WidgetText::from(
        RichText::new(look.detail)
            .size(10.5)
            .color(theme::with_alpha(theme::Palette::TEXT_DIM, 0.9)),
    )
    .into_galley(
        ui,
        Some(egui::TextWrapMode::Wrap),
        text_width,
        egui::TextStyle::Body,
    );
    painter.galley(pos2(text_x, rect.top() + 36.0), galley, Color32::WHITE);

     
    let bar_y = rect.bottom() - 7.0;
    let track = Rect::from_min_max(
        pos2(rect.left() + 18.0, bar_y),
        pos2(rect.right() - 18.0, bar_y + 2.0),
    );
    painter.rect_filled(
        track,
        Rounding::same(1.0),
        theme::with_alpha(theme::Palette::TEXT_DIM, 0.15),
    );
    let fill = Rect::from_min_max(
        track.min,
        pos2(
            track.left() + track.width() * look.remaining,
            track.bottom(),
        ),
    );
    painter.rect_filled(fill, Rounding::same(1.0), theme::with_alpha(color, 0.9));
}

 
 
fn paint_notice(ui: &mut egui::Ui, message: &str, ok: bool) {
    let color = if ok {
        theme::Palette::OK
    } else {
        theme::Palette::ERR
    };
    egui::Frame::none()
        .fill(theme::with_alpha(color, 0.08))
        .stroke(Stroke::new(1.0_f32, theme::with_alpha(color, 0.35)))
        .rounding(Rounding::same(8.0))
        .inner_margin(egui::Margin::symmetric(10.0, 6.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (dot, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                ui.painter().circle_filled(dot.center(), 3.5, color);
                ui.label(
                    RichText::new(message)
                        .size(11.5)
                        .color(theme::Palette::TEXT),
                );
            });
        });
    ui.add_space(6.0);
}

#[cfg(test)]
impl InjectorGui {
     
     
    pub(super) fn stub_gui() -> Self {
        let ctx = Context::default();
        let texture = ctx.load_texture(
            "stub",
            egui::ColorImage::from_rgba_unmultiplied([1, 1], &[255, 255, 255, 255]),
            Default::default(),
        );
        InjectorGui {
            app: InjectorApp::stub(),
            textlogo: texture.clone(),
            sidebar_icon: texture.clone(),
            minimize_icon: texture.clone(),
            close_icon: texture.clone(),
            back_icon: texture.clone(),
            addons_icon: texture.clone(),
            config_icon: texture.clone(),
            friends_icon: texture.clone(),
            macros_icon: texture,
            macros_page: macros::MacrosPage::default(),
            addons_menu_open: true,
            process_menu_open: false,
            drawer_page: DrawerPage::Menu,
            page_entered_at: -10.0,
            page_direction: 1.0,
            addons_opened_at: 0.0,
            last_addon_refresh: 0.0,
            addon_notice: None,
            config_name: String::new(),
            config_notice: None,
            config_modules: Vec::new(),
            config_search: String::new(),
            config_expanded: BTreeSet::new(),
            config_buffers: HashMap::new(),
            config_opened_at: -10.0,
            friends_opened_at: -10.0,
            friends_list: Vec::new(),
            friend_name: String::new(),
            friend_notice: None,
            friends_expanded: BTreeSet::new(),
            toast: None,
            status_key: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{page_slide_offset, PAGE_SLIDE_PX, PAGE_SLIDE_SECS};

     
     
    #[test]
    fn a_page_slides_in_from_the_side_it_came_from() {
        assert!((page_slide_offset(1.0, 0.0) - PAGE_SLIDE_PX).abs() < 0.001);
        assert!((page_slide_offset(-1.0, 0.0) + PAGE_SLIDE_PX).abs() < 0.001);
        assert_eq!(page_slide_offset(1.0, PAGE_SLIDE_SECS), 0.0);
         
        assert_eq!(page_slide_offset(-1.0, PAGE_SLIDE_SECS * 4.0), 0.0);

        let mut previous = PAGE_SLIDE_PX;
        for step in 0..=20 {
            let elapsed = PAGE_SLIDE_SECS * step as f64 / 20.0;
            let offset = page_slide_offset(1.0, elapsed);
            assert!(offset <= previous + 0.001, "offset must only decay");
            assert!(offset >= 0.0, "offset must not cross the slot");
            previous = offset;
        }
    }
}
