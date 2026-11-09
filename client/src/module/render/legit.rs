 
 
 
 
 
 

use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};

 
#[derive(Debug)]
pub struct HudModule {
    pub module: ModuleData,
}

impl HudModule {
    fn new(id: ModuleId, description: &str, settings: Vec<ModuleSetting>) -> Self {
        Self {
            module: ModuleData {
                id,
                description: description.to_string(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings,
            },
        }
    }
}

impl Module for HudModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}

 
macro_rules! hud_module {
    ($fn_name:ident, $id:expr, $desc:expr) => {
        pub fn $fn_name() -> HudModule {
            HudModule::new($id, $desc, vec![])
        }
    };
}

hud_module!(coords, ModuleId::Coords, "Displays XYZ coordinates");
hud_module!(fps, ModuleId::Fps, "Displays FPS");
hud_module!(
    compass,
    ModuleId::Compass,
    "Compass rose with the current heading"
);
hud_module!(
    key_strokes,
    ModuleId::KeyStrokes,
    "Displays keyboard/mouse inputs and CPS"
);
 
 
 
pub fn clock() -> HudModule {
    HudModule::new(
        ModuleId::Clock,
        "A watch face or a digital card with real-world time, date and zone",
        vec![
            ModuleSetting::Choice {
                name: "Style".into(),
                value: 0,
                options: ["Analog", "Digital"]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
            },
            ModuleSetting::Toggle {
                name: "24 Hour".into(),
                value: true,
            },
            ModuleSetting::Toggle {
                name: "Seconds".into(),
                value: true,
            },
            ModuleSetting::Toggle {
                name: "Show Date".into(),
                value: false,
            },
            ModuleSetting::Choice {
                name: "Timezone".into(),
                value: 0,
                options: ["Local", "UTC"].into_iter().map(str::to_string).collect(),
            },
            ModuleSetting::Toggle {
                name: "Smooth Seconds".into(),
                value: true,
            },
            ModuleSetting::Slider {
                name: "Scale".into(),
                value: 1.0,
                min: 0.7,
                max: 1.8,
            },
        ],
    )
}
hud_module!(
    speedometer,
    ModuleId::Speedometer,
    "Displays horizontal movement in blocks per second"
);
hud_module!(
    session,
    ModuleId::Session,
    "Displays how long this client session has been running"
);
hud_module!(
    ping,
    ModuleId::Ping,
    "Displays the connection latency reported by the server"
);
pub fn spotify_now_playing() -> HudModule {
    HudModule::new(
        ModuleId::SpotifyNowPlaying,
        "Shows the song playing in Spotify with album art and a progress bar",
        vec![
            ModuleSetting::Slider {
                name: "Scale".into(),
                value: 1.0,
                min: 0.6,
                max: 2.0,
            },
            ModuleSetting::Slider {
                name: "Corner Radius".into(),
                value: 8.0,
                min: 0.0,
                max: 16.0,
            },
            ModuleSetting::Slider {
                name: "Width".into(),
                value: 272.0,
                min: 160.0,
                max: 420.0,
            },
            ModuleSetting::Slider {
                name: "Background Opacity".into(),
                value: 0.62,
                min: 0.0,
                max: 1.0,
            },
            ModuleSetting::Slider {
                name: "Padding".into(),
                value: 10.0,
                min: 0.0,
                max: 20.0,
            },
            ModuleSetting::Slider {
                name: "Art Size".into(),
                value: 46.0,
                min: 24.0,
                max: 72.0,
            },
            ModuleSetting::Slider {
                name: "Art Corner Radius".into(),
                value: 6.0,
                min: 0.0,
                max: 36.0,
            },
            ModuleSetting::Slider {
                name: "Title Size".into(),
                value: 13.5,
                min: 8.0,
                max: 22.0,
            },
            ModuleSetting::Slider {
                name: "Artist Size".into(),
                value: 10.5,
                min: 7.0,
                max: 18.0,
            },
            ModuleSetting::Slider {
                name: "Progress Height".into(),
                value: 3.0,
                min: 1.0,
                max: 8.0,
            },
            ModuleSetting::Color {
                name: "Accent Color".into(),
                value: [0.114, 0.725, 0.329, 1.0],
            },
            ModuleSetting::Color {
                name: "Text Color".into(),
                value: [0.941, 0.922, 0.922, 1.0],
            },
            ModuleSetting::Toggle {
                name: "Show Background".into(),
                value: true,
            },
            ModuleSetting::Toggle {
                name: "Show Title".into(),
                value: true,
            },
            ModuleSetting::Toggle {
                name: "Show Album Art".into(),
                value: true,
            },
            ModuleSetting::Toggle {
                name: "Show Artist".into(),
                value: true,
            },
            ModuleSetting::Toggle {
                name: "Show Progress Bar".into(),
                value: true,
            },
            ModuleSetting::Toggle {
                name: "Show Time".into(),
                value: true,
            },
            ModuleSetting::Toggle {
                name: "Show Border".into(),
                value: true,
            },
        ],
    )
}

 
 
 
pub fn lyrics() -> HudModule {
    HudModule::new(
        ModuleId::Lyrics,
        "Shows synced lyrics as a HUD card, floating words, or across the Atmosphere sky",
        vec![
            ModuleSetting::Slider {
                name: "Scale".into(),
                value: 1.0,
                min: 0.6,
                max: 2.0,
            },
            ModuleSetting::Slider {
                name: "Corner Radius".into(),
                value: 8.0,
                min: 0.0,
                max: 16.0,
            },
            ModuleSetting::Slider {
                name: "Background Opacity".into(),
                value: 0.55,
                min: 0.0,
                max: 1.0,
            },
            ModuleSetting::Slider {
                name: "Max Width".into(),
                value: 360.0,
                min: 160.0,
                max: 700.0,
            },
            ModuleSetting::Slider {
                name: "Lines Above".into(),
                value: 2.0,
                min: 0.0,
                max: 4.0,
            },
            ModuleSetting::Slider {
                name: "Lines Below".into(),
                value: 2.0,
                min: 0.0,
                max: 4.0,
            },
            ModuleSetting::Slider {
                name: "Slide Speed".into(),
                value: 8.0,
                min: 1.0,
                max: 20.0,
            },
            ModuleSetting::Slider {
                name: "Glow Strength".into(),
                value: 0.8,
                min: 0.0,
                max: 1.0,
            },
            ModuleSetting::Color {
                name: "Highlight Color".into(),
                value: [1.0, 1.0, 1.0, 1.0],
            },
            ModuleSetting::Color {
                name: "Text Color".into(),
                value: [1.0, 1.0, 1.0, 1.0],
            },
            ModuleSetting::Toggle {
                name: "Show Background".into(),
                value: true,
            },
            ModuleSetting::Toggle {
                name: "Word Highlight".into(),
                value: true,
            },
            ModuleSetting::Toggle {
                name: "Show Title".into(),
                value: true,
            },
            ModuleSetting::Toggle {
                name: "Sky Lyrics".into(),
                value: false,
            },
            ModuleSetting::Toggle {
                name: "World Space".into(),
                value: false,
            },
            ModuleSetting::Toggle {
                name: "Also Show HUD".into(),
                value: false,
            },
            ModuleSetting::Slider {
                name: "World Distance".into(),
                value: 7.0,
                min: 2.0,
                max: 20.0,
            },
            ModuleSetting::Slider {
                name: "World Height".into(),
                value: 0.4,
                min: -1.0,
                max: 3.0,
            },
            ModuleSetting::Slider {
                name: "World Size".into(),
                value: 1.0,
                min: 0.5,
                max: 2.5,
            },
        ],
    )
}
hud_module!(
    potion_status,
    ModuleId::PotionStatus,
    "Displays active potion effects"
);
hud_module!(
    armor_status,
    ModuleId::ArmorStatus,
    "A card with each worn piece's icon, name and durability bar"
);
 
 
 
pub fn status() -> HudModule {
    HudModule::new(
        ModuleId::StatusHud,
        "A card with your name and health, armour and hunger bars",
        vec![
            ModuleSetting::Toggle {
                name: "Name".into(),
                value: true,
            },
            ModuleSetting::Toggle {
                name: "Health".into(),
                value: true,
            },
            ModuleSetting::Toggle {
                name: "Armor".into(),
                value: true,
            },
            ModuleSetting::Toggle {
                name: "Food".into(),
                value: true,
            },
            ModuleSetting::Toggle {
                name: "Numbers".into(),
                value: true,
            },
            ModuleSetting::Slider {
                name: "Width".into(),
                value: 148.0,
                min: 110.0,
                max: 240.0,
            },
            ModuleSetting::Slider {
                name: "Bar Height".into(),
                value: 6.0,
                min: 4.0,
                max: 12.0,
            },
        ],
    )
}
 
 
pub fn totem_counter() -> HudModule {
    HudModule::new(
        ModuleId::TotemCounter,
        "Counts the Totems of Undying you are carrying, next to the hotbar",
        vec![ModuleSetting::Toggle {
            name: "Show When Empty".into(),
            value: true,
        }],
    )
}
pub fn radar() -> HudModule {
    HudModule::new(
        ModuleId::Radar,
        "Heading-up radar with player, mob and other entity markers",
        vec![
            ModuleSetting::Slider {
                name: "Range".into(),
                value: 64.0,
                min: 16.0,
                max: 128.0,
            },
            ModuleSetting::Toggle {
                name: "Show Mobs".into(),
                value: true,
            },
            ModuleSetting::Toggle {
                name: "Show Other Entities".into(),
                value: true,
            },
        ],
    )
}
 
 
 
pub fn module_list() -> HudModule {
    let mut module = HudModule::new(
        ModuleId::ModuleList,
        "Enabled modules in the top-right corner, with an optional Infered logo",
        vec![
            ModuleSetting::Toggle {
                name: "Show Logo".into(),
                value: true,
            },
            ModuleSetting::Slider {
                name: "Logo Size".into(),
                value: 135.0,
                min: 60.0,
                max: 220.0,
            },
            ModuleSetting::Slider {
                name: "Logo Opacity".into(),
                value: 1.0,
                min: 0.1,
                max: 1.0,
            },
            ModuleSetting::Toggle {
                name: "Show Background".into(),
                value: true,
            },
            ModuleSetting::Slider {
                name: "Background Opacity".into(),
                value: 0.76,
                min: 0.0,
                max: 1.0,
            },
            ModuleSetting::Toggle {
                name: "Show Border".into(),
                value: true,
            },
            ModuleSetting::Toggle {
                name: "Accent Bar".into(),
                value: true,
            },
            ModuleSetting::Toggle {
                name: "Show Dots".into(),
                value: false,
            },
            ModuleSetting::Toggle {
                name: "Show Count".into(),
                value: false,
            },
            ModuleSetting::Toggle {
                name: "Lowercase".into(),
                value: false,
            },
            ModuleSetting::Color {
                name: "Text Color".into(),
                value: [0.941, 0.922, 0.922, 1.0],
            },
            ModuleSetting::Slider {
                name: "Text Size".into(),
                value: 14.0,
                min: 8.0,
                max: 24.0,
            },
            ModuleSetting::Slider {
                name: "Row Height".into(),
                value: 20.0,
                min: 14.0,
                max: 32.0,
            },
            ModuleSetting::Slider {
                name: "Row Spacing".into(),
                value: 2.0,
                min: 0.0,
                max: 10.0,
            },
            ModuleSetting::Slider {
                name: "Right Margin".into(),
                value: 10.0,
                min: 0.0,
                max: 40.0,
            },
            ModuleSetting::Slider {
                name: "Top Offset".into(),
                value: 0.0,
                min: 0.0,
                max: 200.0,
            },
            ModuleSetting::Choice {
                name: "Sort".into(),
                value: 0,
                options: ["Length", "Name", "Category"]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
            },
        ],
    );
    module.module.enabled = true;
    module
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clock_ships_a_local_analog_face_with_its_switches() {
        let module = clock();
        let data = &module.module;
        assert_eq!(data.id, ModuleId::Clock);

        let toggle = |name| {
            data.get_setting(name)
                .and_then(ModuleSetting::get_toggle_value)
                .unwrap_or_else(|| panic!("missing toggle {name}"))
        };
        assert!(toggle("24 Hour"));
        assert!(toggle("Seconds"));
        assert!(!toggle("Show Date"), "the date is opt-in");
        assert!(toggle("Smooth Seconds"), "the sweep hand is the default");

         
         
        for (name, options) in [
            ("Style", ["Analog", "Digital"]),
            ("Timezone", ["Local", "UTC"]),
        ] {
            let setting = data
                .get_setting(name)
                .unwrap_or_else(|| panic!("missing {name}"));
            match setting {
                ModuleSetting::Choice {
                    value,
                    options: list,
                    ..
                } => {
                    assert_eq!(*value, 0, "{name} starts on its first option");
                    assert_eq!(list.iter().map(String::as_str).collect::<Vec<_>>(), options);
                }
                other => panic!("{name} is not a choice: {other:?}"),
            }
        }

        let scale = data
            .get_setting("Scale")
            .and_then(ModuleSetting::get_slider_value)
            .expect("Scale slider");
        assert_eq!(scale, 1.0);
    }

    #[test]
    fn lyrics_ships_world_space_off_with_its_own_defaults() {
        let module = lyrics();
        let data = &module.module;
        let toggle = |name| {
            data.get_setting(name)
                .and_then(ModuleSetting::get_toggle_value)
                .unwrap_or_else(|| panic!("missing toggle {name}"))
        };
        let slider = |name| {
            data.get_setting(name)
                .and_then(ModuleSetting::get_slider_value)
                .unwrap_or_else(|| panic!("missing slider {name}"))
        };

        assert!(!toggle("Sky Lyrics"), "sky mode is opt-in");
        assert!(!toggle("World Space"), "world mode is opt-in");
        assert!(!toggle("Also Show HUD"), "world mode replaces the card");
        assert_eq!(slider("World Distance"), 7.0);
        assert_eq!(slider("World Height"), 0.4);
        assert_eq!(slider("World Size"), 1.0);
         
         
        for removed in ["World Spread", "World Trail", "World Fade", "World Beams"] {
            assert!(
                data.get_setting(removed).is_none(),
                "{removed} should no longer exist"
            );
        }
    }

    #[test]
    fn module_list_ships_enabled_with_its_visual_settings() {
        let module = module_list();
        let data = &module.module;
        assert_eq!(data.id, ModuleId::ModuleList);
        assert_eq!(data.category, ModuleCategory::Visuals);
        assert!(data.enabled, "the list replaces the old always-on HUD");

        let toggle = |name| {
            data.get_setting(name)
                .and_then(ModuleSetting::get_toggle_value)
                .unwrap_or_else(|| panic!("missing toggle {name}"))
        };
        let slider = |name| {
            data.get_setting(name)
                .and_then(ModuleSetting::get_slider_value)
                .unwrap_or_else(|| panic!("missing slider {name}"))
        };

        assert!(toggle("Show Logo"));
        assert_eq!(slider("Logo Size"), 135.0);
        assert_eq!(slider("Logo Opacity"), 1.0);
        assert!(toggle("Show Background"));
        assert!(toggle("Show Border"));
        assert!(toggle("Accent Bar"));
        assert!(!toggle("Show Dots"));
        assert!(!toggle("Show Count"));
        assert!(!toggle("Lowercase"));
        assert_eq!(slider("Text Size"), 14.0);
        assert_eq!(slider("Row Height"), 20.0);
        assert_eq!(slider("Row Spacing"), 2.0);
        assert_eq!(slider("Right Margin"), 10.0);
        assert_eq!(slider("Top Offset"), 0.0);

        let sort = data.get_setting("Sort").expect("Sort choice");
        match sort {
            ModuleSetting::Choice { value, options, .. } => {
                assert_eq!(*value, 0);
                assert_eq!(
                    options.iter().map(String::as_str).collect::<Vec<_>>(),
                    ["Length", "Name", "Category"]
                );
            }
            other => panic!("Sort is not a choice: {other:?}"),
        }
    }
}

 
 
 
pub fn menu_hud() -> HudModule {
    let mut module = HudModule::new(
        ModuleId::MenuHud,
        "Opens and closes the ClickGUI menu",
        vec![],
    );
    module.module.key_bind = KeyboardKey::KeyRShift;
    module
}

 
 
 
 
pub fn top_bar() -> HudModule {
    HudModule::new(
        ModuleId::TopBar,
        "A row of capsules: clock, FPS, ping, direction and whatever else you want",
        vec![
            ModuleSetting::Slider {
                name: "Bar Height".into(),
                value: 26.0,
                min: 18.0,
                max: 40.0,
            },
            ModuleSetting::Toggle {
                name: "Show Brand".into(),
                value: true,
            },
            ModuleSetting::Toggle {
                name: "Show Wordmark".into(),
                value: true,
            },
            ModuleSetting::Toggle {
                name: "Show Time".into(),
                value: true,
            },
            ModuleSetting::Toggle {
                name: "Show Date".into(),
                value: false,
            },
            ModuleSetting::Toggle {
                name: "Show FPS".into(),
                value: true,
            },
            ModuleSetting::Toggle {
                name: "Show Ping".into(),
                value: true,
            },
            ModuleSetting::Toggle {
                name: "Show Direction".into(),
                value: true,
            },
            ModuleSetting::Toggle {
                name: "Show Coordinates".into(),
                value: false,
            },
            ModuleSetting::Toggle {
                name: "Show Speed".into(),
                value: false,
            },
            ModuleSetting::Toggle {
                name: "Show Module Count".into(),
                value: true,
            },
        ],
    )
}
