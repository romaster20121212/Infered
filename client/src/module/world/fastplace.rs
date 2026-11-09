use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
struct PlaceState {
    last_place: Option<Instant>,
}

 
 
 
#[derive(Debug)]
pub struct FastPlaceModule {
    pub module: ModuleData,
    state: Mutex<PlaceState>,
}

impl FastPlaceModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::FastPlace,
                description: "Reduces block-placement delay".to_string(),
                category: ModuleCategory::World,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Delay".to_string(),
                    value: 100.0,
                    min: 20.0,
                    max: 400.0,
                }],
            },
            state: Mutex::new(PlaceState::default()),
        }
    }
}

impl Module for FastPlaceModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        *self.state.lock().unwrap() = PlaceState::default();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
         
        if crate::graphic::input::GUI_OPEN.load(std::sync::atomic::Ordering::Relaxed) {
            return Ok(());
        }
        let right_down = crate::graphic::input::MOUSE_STATE
            .lock()
            .map(|mouse| mouse.right_down)
            .unwrap_or(false);
        if !right_down {
            *self.state.lock().unwrap() = PlaceState::default();
            return Ok(());
        }

        let minecraft = minecraft();
        if minecraft.current_screen().is_open() {
            return Ok(());
        }
        let (Some(player), Some(game_mode)) = (minecraft.player()?, minecraft.game_mode()?) else {
            return Ok(());
        };

        let delay = Duration::from_millis(
            self.module
                .get_setting("Delay")
                .and_then(|setting| setting.get_slider_value())
                .unwrap_or(100.0) as u64,
        );
        let mut state = self.state.lock().unwrap();
        if state.last_place.is_some_and(|last| last.elapsed() < delay) {
            return Ok(());
        }
         
        if game_mode.use_item(&player).is_ok() {
            state.last_place = Some(Instant::now());
        }
        Ok(())
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}
