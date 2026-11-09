 
 
 
#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod friends;
mod gui;
mod inject;
mod modules;
mod platform;
mod tui;

use log::{error, LevelFilter};

fn main() {
    reattach_console();

    let args: Vec<String> = std::env::args().collect();

     
    if args.iter().any(|a| a == "--list") {
        for proc in platform::find_minecraft_processes() {
            println!("{}\t{}", proc.pid, proc.info);
        }
        return;
    }

     
     
     
    let interactive_gui = !args.iter().any(|a| a == "--tui") && injection_target(&args).is_none();

    if !platform::is_elevated() {
        if interactive_gui {
            #[cfg(windows)]
            if request_elevation() {
                 
                 
                std::process::exit(0);
            }
            eprintln!("{}", elevation_hint());
            std::process::exit(1);
        }
        eprintln!("{}", elevation_hint());
        std::process::exit(1);
    }

    let level = if cfg!(any(debug_assertions, feature = "devtest")) {
        LevelFilter::Debug
    } else {
        LevelFilter::Warn
    };
    let _ = protocol::init_file_logger("app.log", level);

     
    if let Some(pid) = injection_target(&args) {
        std::process::exit(run_headless_injection(pid));
    }

    if args.iter().any(|a| a == "--tui") {
        tui::run_tui();
        return;
    }

    if let Err(e) = gui::run() {
        error!("GUI terminated with an error: {e}");
        eprintln!("Error: {e}");
    }
}

 
 
 
 
#[cfg(windows)]
fn reattach_console() {
    if std::env::args().len() > 1 {
        unsafe {
            use windows_sys::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
            AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }
}

 
#[cfg(not(windows))]
fn reattach_console() {}

 
 
 
#[cfg(windows)]
fn request_elevation() -> bool {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

     
     
    let Some(exe) = std::env::current_exe().ok() else {
        return false;
    };
    let params = std::env::args().skip(1).collect::<Vec<_>>();
    let params_line = params.join(" ");

    let wide = |s: &str| -> Vec<u16> {
        OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    };

    let exe_w = wide(&exe.to_string_lossy());
    let params_w = wide(&params_line);
    let op_w = wide("runas");

    let result = unsafe {
        ShellExecuteW(
            0,
            op_w.as_ptr(),
            exe_w.as_ptr(),
            if params.is_empty() {
                std::ptr::null()
            } else {
                params_w.as_ptr()
            },
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };

     
     
    result > 32
}

 
fn injection_target(args: &[String]) -> Option<u32> {
    let index = args.iter().position(|a| a == "--inject")?;
    args.get(index + 1)?.parse().ok()
}

 
fn run_headless_injection(pid: u32) -> i32 {
    match inject::inject(pid) {
        Ok(()) => {
            println!("injected into process {pid}");
            0
        }
        Err(e) => {
            eprintln!("injection into process {pid} failed: {e}");
            1
        }
    }
}

 
 
fn elevation_hint() -> &'static str {
    #[cfg(windows)]
    {
        "This program must run as Administrator (right click → Run as administrator)."
    }
    #[cfg(not(windows))]
    {
        "This program must run with root privileges: sudo ./injector"
    }
}
