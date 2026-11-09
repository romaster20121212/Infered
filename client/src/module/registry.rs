 

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use dashmap::DashMap;
use log::{error, warn};

use crate::module::{KeyboardKey, Module, ModuleId, ModuleSetting, ModuleType};
use crate::net::packet::{Packet, PacketAction};

 
pub type ModuleHandle = Arc<Mutex<ModuleType>>;

 
 
 
 
 
 
const MAX_CONSECUTIVE_TICK_FAILURES: u32 = 40;

 
 
struct ModuleDefaults {
    key_bind: KeyboardKey,
    settings: Vec<ModuleSetting>,
}

 
 
#[derive(Default)]
pub struct ModuleRegistry {
    modules: DashMap<ModuleId, ModuleHandle>,
     
    defaults: DashMap<ModuleId, ModuleDefaults>,
     
     
    tick_failures: DashMap<ModuleId, u32>,
     
     
     
    active: AtomicBool,
}

impl ModuleRegistry {
     
    pub fn new() -> Self {
        Self::default()
    }

     
     
    pub fn register<M>(&self, module: M)
    where
        M: Module + Send + Sync + 'static,
    {
        let mut module: ModuleType = Box::new(module);
        let data = module.get_module_data();
        let id = data.id;
         
         
        let factory_key = if matches!(id, ModuleId::MenuHud | ModuleId::Freelook) {
            data.key_bind
        } else {
            KeyboardKey::KeyNone
        };
        self.defaults.insert(
            id,
            ModuleDefaults {
                key_bind: factory_key,
                settings: data.settings.clone(),
            },
        );
        module.get_module_data_mut().key_bind = factory_key;
        self.modules.insert(id, Arc::new(Mutex::new(module)));
    }

     
     
    pub fn reset_settings(&self) {
        for entry in self.modules.iter() {
            let Some(defaults) = self.defaults.get(entry.key()) else {
                continue;
            };
            let Ok(mut module) = entry.value().lock() else {
                continue;
            };
            if module.get_module_data().enabled && self.is_active() {
                let _ = module.on_stop();
            }
            let data = module.get_module_data_mut();
            data.key_bind = defaults.key_bind;
            data.settings = defaults.settings.clone();
            data.enabled = false;
        }
    }

     
    pub fn reset_module_settings(&self, id: ModuleId) {
        let Some(handle) = self.get(id) else {
            return;
        };
        let Some(defaults) = self.defaults.get(&id) else {
            return;
        };
        let Ok(mut module) = handle.lock() else {
            return;
        };
        if module.get_module_data().enabled && self.is_active() {
            let _ = module.on_stop();
        }
        let data = module.get_module_data_mut();
        data.key_bind = defaults.key_bind;
        data.settings = defaults.settings.clone();
        data.enabled = false;
    }

     
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

     
     
     
     
    pub fn set_active(&self, active: bool) {
        if self.active.swap(active, Ordering::SeqCst) == active {
            return;  
        }
         
         
        self.tick_failures.clear();
        for handle in self.handles() {
            let Ok(module) = handle.lock() else {
                continue;
            };
            if !module.get_module_data().enabled {
                continue;
            }
            let result = if active {
                module.on_start()
            } else {
                module.on_world_suspend()
            };
            if let Err(e) = result {
                let name = module.get_module_data().name();
                let verb = if active { "start" } else { "stop" };
                error!("module '{name}' failed to {verb} on world transition: {e}");
            }
        }
    }

     
     
     
    pub fn restore_inactive_render_state(&self) {
        for id in [
            ModuleId::Fullbright,
            ModuleId::Fov,
            ModuleId::NoFovEffects,
            ModuleId::EntityScale,
            ModuleId::Zoom,
            ModuleId::ItemScale,
            ModuleId::CustomSky,
            ModuleId::InvisibleShield,
            ModuleId::SwingEcho,
            ModuleId::HandOutline,
            ModuleId::HandAura,
            ModuleId::InvisFire,
            ModuleId::CustomCrosshair,
            ModuleId::NoParticles,
            ModuleId::TotemPop,
            ModuleId::RenderDistanceCap,
            ModuleId::EntityDistanceCap,
            ModuleId::EntityShadowsOff,
            ModuleId::SmoothLightingOff,
            ModuleId::BiomeBlendCap,
            ModuleId::FramerateCap,
            ModuleId::VsyncOff,
            ModuleId::SimulationDistanceCap,
        ] {
            let Some(handle) = self.get(id) else {
                continue;
            };
            let Ok(module) = handle.lock() else {
                continue;
            };
            if let Err(error) = module.on_world_inactive() {
                log::debug!(
                    "render state cleanup for {} failed: {error}",
                    id.display_name()
                );
            }
        }
    }

     
    pub fn get(&self, id: ModuleId) -> Option<ModuleHandle> {
        self.modules.get(&id).map(|entry| Arc::clone(entry.value()))
    }

     
     
    pub fn handles(&self) -> Vec<ModuleHandle> {
        self.modules
            .iter()
            .map(|entry| Arc::clone(entry.value()))
            .collect()
    }

     
    pub fn by_id(&self) -> HashMap<ModuleId, ModuleHandle> {
        self.modules
            .iter()
            .map(|entry| (*entry.key(), Arc::clone(entry.value())))
            .collect()
    }

     
     
     
    pub fn tick(&self) {
         
         
        let auto_eat = self.get(ModuleId::AutoEat);
        if let Some(handle) = &auto_eat {
            self.tick_module(handle);
        }
        for handle in self.handles() {
            if auto_eat
                .as_ref()
                .is_some_and(|eat| Arc::ptr_eq(eat, &handle))
            {
                continue;
            }
            self.tick_module(&handle);
        }
    }

     
     
     
     
     
     
    pub fn input_phase(&self) {
        if !self.is_active() {
            return;
        }
        for handle in self.handles() {
            let Ok(module) = handle.lock() else {
                continue;
            };
            if !module.get_module_data().enabled {
                continue;
            }
            if let Err(error) = module.on_input_phase() {
                warn!(
                    "module '{}' input phase failed: {error}",
                    module.get_module_data().name()
                );
            }
        }
    }

    fn tick_module(&self, handle: &ModuleHandle) {
        let Ok(mut module) = handle.lock() else {
            return;
        };
        if !module.get_module_data().enabled {
            return;
        }
        let id = module.get_module_data().id;
        match module.on_tick() {
            Ok(()) => {
                self.tick_failures.remove(&id);
            }
            Err(e) => {
                let name = module.get_module_data().name();
                let failures = {
                    let mut entry = self.tick_failures.entry(id).or_insert(0);
                    *entry += 1;
                    *entry
                };
                if failures < MAX_CONSECUTIVE_TICK_FAILURES {
                     
                     
                     
                    if failures == 1 {
                        warn!("module '{name}' tick failed, retrying: {e}");
                    }
                    return;
                }
                error!("module '{name}' failed {failures} ticks in a row, disabling it: {e}");
                if let Err(e) = module.on_stop() {
                    error!("module '{name}' also failed to stop: {e}");
                }
                 
                 
                 
                module.get_module_data_mut().set_enabled(false);
                self.tick_failures.remove(&id);
            }
        }
    }

     
    pub fn camera_setup(&self) {
        for handle in self.handles() {
            let Ok(module) = handle.lock() else {
                continue;
            };
            if module.get_module_data().enabled {
                module.on_camera_setup();
            }
        }
    }

     
    pub fn camera_setup_object(&self, camera: &jni::objects::JObject) -> anyhow::Result<()> {
        let freecam_enabled = self
            .get(ModuleId::Freecam)
            .and_then(|handle| {
                handle
                    .lock()
                    .ok()
                    .map(|module| module.get_module_data().enabled)
            })
            .unwrap_or(false);
        for handle in self.handles() {
            let Ok(module) = handle.lock() else {
                continue;
            };
             
             
            let id = module.get_module_data().id;
            if !camera_owner_allowed(id, freecam_enabled) {
                continue;
            }
            if module.get_module_data().enabled || id == ModuleId::Freelook {
                module.on_camera_setup_object(camera)?;
            }
        }
        Ok(())
    }

     
    pub fn render(&self) {
        for handle in self.handles() {
            let Ok(module) = handle.lock() else {
                continue;
            };
            if module.get_module_data().enabled {
                module.on_render();
            }
        }
    }

     
     
     
     
     
    pub fn handle_packet(&self, packet: &mut Packet) -> PacketAction {
        if !self.is_active() {
            return PacketAction::Forward;
        }
        for handle in self.handles() {
            let Ok(module) = handle.lock() else {
                continue;
            };
            if !module.get_module_data().enabled {
                continue;
            }
            if module.handle_packet(packet) == PacketAction::Cancel {
                return PacketAction::Cancel;
            }
        }
        PacketAction::Forward
    }
}

fn camera_owner_allowed(id: ModuleId, freecam: bool) -> bool {
     
    id != ModuleId::Freelook || !freecam
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_does_not_reserve_camera_and_freecam_still_overrides_freelook() {
        for freecam in [false, true] {
            assert_eq!(camera_owner_allowed(ModuleId::Freelook, freecam), !freecam);
            assert!(camera_owner_allowed(ModuleId::Scaffold, freecam));
            assert!(camera_owner_allowed(ModuleId::Freecam, freecam));
        }
    }
    use crate::module::{KeyboardKey, ModuleCategory, ModuleData};
    use std::sync::atomic::AtomicBool;

     
    #[derive(Debug)]
    struct FailingModule {
        data: ModuleData,
    }

     
     
    #[derive(Debug)]
    struct SwitchableModule {
        data: ModuleData,
        fail: Arc<AtomicBool>,
    }

    fn data() -> ModuleData {
        ModuleData {
            id: ModuleId::NoFall,
            description: "test module".to_string(),
            category: ModuleCategory::Movement,
            key_bind: KeyboardKey::KeyNone,
             
            enabled: true,
            settings: vec![],
        }
    }

    impl Module for FailingModule {
        fn on_start(&self) -> anyhow::Result<()> {
            Ok(())
        }
        fn on_stop(&self) -> anyhow::Result<()> {
            Ok(())
        }
        fn on_tick(&self) -> anyhow::Result<()> {
            anyhow::bail!("boom")
        }
        fn get_module_data(&self) -> &ModuleData {
            &self.data
        }
        fn get_module_data_mut(&mut self) -> &mut ModuleData {
            &mut self.data
        }
    }

    impl Module for SwitchableModule {
        fn on_start(&self) -> anyhow::Result<()> {
            Ok(())
        }
        fn on_stop(&self) -> anyhow::Result<()> {
            Ok(())
        }
        fn on_tick(&self) -> anyhow::Result<()> {
            if self.fail.load(Ordering::Relaxed) {
                anyhow::bail!("boom")
            }
            Ok(())
        }
        fn get_module_data(&self) -> &ModuleData {
            &self.data
        }
        fn get_module_data_mut(&mut self) -> &mut ModuleData {
            &mut self.data
        }
    }

     
    fn is_enabled(registry: &ModuleRegistry) -> bool {
        registry
            .get(ModuleId::NoFall)
            .expect("the module is registered")
            .lock()
            .expect("the module lock is clean")
            .get_module_data()
            .enabled
    }

    #[test]
    fn freelook_has_f6_factory_binding_and_reset_restores_it() {
        let registry = ModuleRegistry::new();
        registry.register(crate::module::misc::freelook::FreelookModule::new());
        let handle = registry.get(ModuleId::Freelook).unwrap();
        assert_eq!(
            handle.lock().unwrap().get_module_data().key_bind,
            KeyboardKey::KeyF6
        );
        handle.lock().unwrap().get_module_data_mut().key_bind = KeyboardKey::KeyG;
        registry.reset_settings();
        assert_eq!(
            handle.lock().unwrap().get_module_data().key_bind,
            KeyboardKey::KeyF6
        );
    }

    #[test]
    fn reset_settings_restores_every_setting_and_disables_the_module() {
        let registry = ModuleRegistry::new();
        registry.register(crate::module::player::totempanic::TotemPanicModule::new());
        let handle = registry.get(ModuleId::TotemPanic).unwrap();
        {
            let mut module = handle.lock().unwrap();
            let data = module.get_module_data_mut();
            data.get_setting_mut("Health")
                .unwrap()
                .set_slider_value(12.0);
            data.enabled = true;
        }
        registry.reset_settings();
        let module = handle.lock().unwrap();
        let data = module.get_module_data();
        assert_eq!(
            data.get_setting("Health")
                .and_then(|s| s.get_slider_value()),
            Some(6.0)
        );
        assert!(!data.enabled);
    }

    #[test]
    fn a_transient_tick_failure_keeps_the_module_armed() {
        let registry = ModuleRegistry::new();
        registry.register(FailingModule { data: data() });

        registry.tick();

        assert!(
            is_enabled(&registry),
            "one failed tick must not disable a module"
        );
    }

    #[test]
    fn a_sustained_tick_failure_stands_the_module_down() {
        let registry = ModuleRegistry::new();
        registry.register(FailingModule { data: data() });

        for _ in 0..MAX_CONSECUTIVE_TICK_FAILURES {
            registry.tick();
        }

        assert!(
            !is_enabled(&registry),
            "a module that fails every tick must eventually be disabled"
        );
    }

    #[test]
    fn a_successful_tick_resets_the_failure_run() {
        let registry = ModuleRegistry::new();
        let fail = Arc::new(AtomicBool::new(true));
        registry.register(SwitchableModule {
            data: data(),
            fail: Arc::clone(&fail),
        });

         
        for _ in 0..MAX_CONSECUTIVE_TICK_FAILURES - 1 {
            registry.tick();
        }
        assert!(is_enabled(&registry));

         
        fail.store(false, Ordering::Relaxed);
        registry.tick();

         
         
        fail.store(true, Ordering::Relaxed);
        for _ in 0..MAX_CONSECUTIVE_TICK_FAILURES - 1 {
            registry.tick();
        }
        assert!(
            is_enabled(&registry),
            "a successful tick must reset the consecutive-failure count"
        );

        registry.tick();
        assert!(!is_enabled(&registry));
    }

    #[test]
    fn a_world_transition_clears_the_failure_run() {
        let registry = ModuleRegistry::new();
        registry.register(FailingModule { data: data() });

        for _ in 0..MAX_CONSECUTIVE_TICK_FAILURES - 1 {
            registry.tick();
        }

         
        registry.set_active(true);
        registry.set_active(false);
        registry.set_active(true);

        registry.tick();
        assert!(
            is_enabled(&registry),
            "a world transition must clear the failure run"
        );
    }
}
