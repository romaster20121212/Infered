 

 
 
pub fn install_signal_handlers() {
    #[cfg(unix)]
    unix::install();
}

#[cfg(unix)]
mod unix {
    use std::sync::atomic::{AtomicBool, Ordering};

    use log::info;

     
    static INSTALLED: AtomicBool = AtomicBool::new(false);

     
    pub fn install() {
        if INSTALLED.swap(true, Ordering::SeqCst) {
            return;
        }

         
         
         
        extern "C" fn handle_signal(_sig: libc::c_int) {
            crate::shutdown();
            std::process::exit(0);
        }

         
         
        unsafe {
            libc::signal(
                libc::SIGTERM,
                handle_signal as *const () as libc::sighandler_t,
            );
            libc::signal(
                libc::SIGINT,
                handle_signal as *const () as libc::sighandler_t,
            );
        }

        info!("signal handlers installed");
    }
}
