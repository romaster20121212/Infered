use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId};
use std::sync::atomic::{AtomicBool, Ordering};

 
 
static PANIC_REQUESTED: AtomicBool = AtomicBool::new(false);

 
 
 
 
#[derive(Debug)]
pub struct PanicModule {
    pub module: ModuleData,
}

impl PanicModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::Panic,
                description:
                    "Like the menu's Panic button: disables every module and closes the overlay"
                        .to_string(),
                category: ModuleCategory::Misc,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![],
            },
        }
    }

     
     
     
     
    fn press_panic_button() {
        if claim_panic_request() {
            std::thread::spawn(crate::graphic::ui_engine::call_panic);
        }
    }
}

 
 
fn claim_panic_request() -> bool {
    !PANIC_REQUESTED.swap(true, Ordering::SeqCst)
}

impl Module for PanicModule {
    fn on_start(&self) -> anyhow::Result<()> {
         
         
        Self::press_panic_button();
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
         
         
        PANIC_REQUESTED.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
         
         
         
        Self::press_panic_button();
        Ok(())
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}

#[cfg(test)]
mod tests {
    use super::{claim_panic_request, PANIC_REQUESTED};
    use std::sync::atomic::Ordering;

     
     
     
    #[test]
    fn a_panic_request_is_claimed_once_then_rearmed_on_stop() {
        PANIC_REQUESTED.store(false, Ordering::SeqCst);
        assert!(
            claim_panic_request(),
            "the first request must claim the run"
        );
        assert!(
            !claim_panic_request(),
            "a follow-up tick must not spawn a second panic run"
        );
         
        PANIC_REQUESTED.store(false, Ordering::SeqCst);
        assert!(claim_panic_request(), "a re-enabled Panic must fire again");
        PANIC_REQUESTED.store(false, Ordering::SeqCst);
    }
}
