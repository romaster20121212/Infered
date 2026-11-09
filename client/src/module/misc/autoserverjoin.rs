use crate::mapping::client::screen::Screen;
use crate::mapping::{FieldType, MappedObject, MinecraftClassType};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::{mapping, minecraft};
use jni::objects::{GlobalRef, JObject, JString, JValue};
use std::sync::Mutex;
use std::time::{Duration, Instant};

 
const DEFAULT_DELAY_SECONDS: f32 = 5.0;
const MAX_DELAY_SECONDS: f32 = 60.0;

 
#[derive(Debug)]
struct RememberedServer {
     
    data: GlobalRef,
     
    ip: String,
}

 
#[derive(Debug, PartialEq, Eq)]
enum Step {
     
    Idle,
     
    StartCountdown,
     
    Wait,
     
    Join,
}

 
 
 
fn next_step(
    kicked: bool,
    can_rejoin: bool,
    countdown_started: Option<Instant>,
    now: Instant,
    delay: Duration,
) -> Step {
    if !kicked || !can_rejoin {
        return Step::Idle;
    }
    match countdown_started {
        None => Step::StartCountdown,
        Some(started) if now.duration_since(started) < delay => Step::Wait,
        Some(_) => Step::Join,
    }
}

 
fn server_ip(data: &GlobalRef) -> anyhow::Result<String> {
    mapping().in_frame(|| {
        let ip = mapping()
            .get_field(
                MinecraftClassType::ServerData,
                data.as_obj(),
                "ip",
                FieldType::String,
            )?
            .l()?;
        let ip = JString::from(ip);
        let java_str = mapping().get_env()?.get_string(&ip)?;
        Ok(String::from(java_str))
    })
}

 
 
 
 
#[derive(Debug)]
pub struct AutoServerJoinModule {
    pub module: ModuleData,
    last_server: Mutex<Option<RememberedServer>>,
     
    countdown: Mutex<Option<Instant>>,
}

impl AutoServerJoinModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::AutoServerJoin,
                description: "Rejoins the last server after a kick".to_string(),
                category: ModuleCategory::Misc,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Delay".to_string(),
                    value: DEFAULT_DELAY_SECONDS,
                    min: 0.0,
                    max: MAX_DELAY_SECONDS,
                }],
            },
            last_server: Mutex::new(None),
            countdown: Mutex::new(None),
        }
    }

     
    fn delay(&self) -> Duration {
        let seconds = self
            .module
            .get_setting("Delay")
            .and_then(|setting| setting.get_slider_value())
            .filter(|value| value.is_finite())
            .unwrap_or(DEFAULT_DELAY_SECONDS)
            .clamp(0.0, MAX_DELAY_SECONDS);
        Duration::from_secs_f32(seconds)
    }

    fn set_countdown(&self, started: Option<Instant>) {
        *self.countdown.lock().unwrap_or_else(|p| p.into_inner()) = started;
    }

     
    fn remember_server(&self) -> anyhow::Result<()> {
        let Some(data) = minecraft().current_server()? else {
            return Ok(());
        };
        let mut last = self.last_server.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(known) = last.as_ref() {
            if mapping().get_env()?.is_same_object(&known.data, &data)? {
                return Ok(());
            }
        }
        let ip = server_ip(&data)?;
        *last = Some(RememberedServer { data, ip });
        Ok(())
    }

     
    fn join(&self) -> anyhow::Result<()> {
        let (data, ip) = {
            let last = self.last_server.lock().unwrap_or_else(|p| p.into_inner());
            match last.as_ref() {
                Some(known) => (known.data.clone(), known.ip.clone()),
                None => return Ok(()),
            }
        };
        mapping().in_frame(|| {
            let java_ip: JObject = mapping().get_env()?.new_string(ip.as_str())?.into();
            let address = mapping()
                .call_static_method(
                    MinecraftClassType::ServerAddress,
                    "parseString",
                    &[JValue::Object(&java_ip)],
                )?
                .l()?;
            let no_screen = JObject::null();
            let no_transfer = JObject::null();
            mapping().call_static_method(
                MinecraftClassType::ConnectScreen,
                "startConnecting",
                &[
                     
                    JValue::Object(&no_screen),
                    JValue::Object(minecraft().jni_ref().as_obj()),
                    JValue::Object(&address),
                    JValue::Object(data.as_obj()),
                     
                    JValue::Bool(0),
                     
                    JValue::Object(&no_transfer),
                ],
            )?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_happens_unless_kicked_with_a_remembered_server() {
        let now = Instant::now();
        let delay = Duration::from_secs(5);
        assert_eq!(next_step(false, true, None, now, delay), Step::Idle);
        assert_eq!(next_step(true, false, None, now, delay), Step::Idle);
    }

    #[test]
    fn countdown_starts_on_kick_then_joins_after_the_delay() {
        let start = Instant::now();
        let delay = Duration::from_secs(5);
        assert_eq!(
            next_step(true, true, None, start, delay),
            Step::StartCountdown
        );
        assert_eq!(
            next_step(
                true,
                true,
                Some(start),
                start + Duration::from_secs(4),
                delay
            ),
            Step::Wait
        );
        assert_eq!(
            next_step(true, true, Some(start), start + delay, delay),
            Step::Join
        );
    }

    #[test]
    fn default_delay_is_five_seconds() {
        let module = AutoServerJoinModule::new();
        assert_eq!(module.delay(), Duration::from_secs(5));
    }
}

impl Module for AutoServerJoinModule {
    fn on_start(&self) -> anyhow::Result<()> {
        log::info!("[AutoServerJoin] enabled: will rejoin the last server after a kick");
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        self.set_countdown(None);
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let minecraft = minecraft();
        if minecraft.in_world() {
            self.remember_server()?;
            self.set_countdown(None);
            return Ok(());
        }

        let kicked = minecraft.current_screen() == Screen::Disconnected;
        let can_rejoin = self
            .last_server
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .is_some();
        let started = *self.countdown.lock().unwrap_or_else(|p| p.into_inner());
        let now = Instant::now();
        match next_step(kicked, can_rejoin, started, now, self.delay()) {
            Step::Idle => self.set_countdown(None),
            Step::StartCountdown => self.set_countdown(Some(now)),
            Step::Wait => {}
            Step::Join => {
                self.set_countdown(None);
                if let Err(err) = self.join() {
                    log::warn!("[AutoServerJoin] rejoin failed: {err:#}");
                }
            }
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
