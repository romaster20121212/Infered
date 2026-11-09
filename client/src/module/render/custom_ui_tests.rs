use super::*;

fn slider(data: &ModuleData, name: &str) -> f32 {
    data.get_setting(name)
        .and_then(ModuleSetting::get_slider_value)
        .unwrap_or_else(|| panic!("missing slider {name}"))
}

fn toggle(data: &ModuleData, name: &str) -> bool {
    data.get_setting(name)
        .and_then(ModuleSetting::get_toggle_value)
        .unwrap_or_else(|| panic!("missing toggle {name}"))
}

#[test]
fn ships_off_with_only_pause_menu_and_hotbar_settings() {
    let module = CustomUiModule::new();
    let data = &module.module;
    assert_eq!(data.id, ModuleId::CustomUi);
    assert_eq!(data.category, ModuleCategory::Visuals);
    assert!(!data.enabled, "the custom UI is opt-in, never always-on");

     
     
     
    let names: Vec<&str> = data.settings.iter().map(|setting| setting.name()).collect();
    assert_eq!(
        names,
        [
            "Pause Menu",
            "Pause Animations",
            "Pause Speed",
            "Backdrop Dim",
            "Embers",
            "Ember Count",
            "Hotbar",
            "Hotbar Scale",
            "Slide Speed",
            "Hotbar Glow",
            "Hotbar Numbers",
            "Item Name",
            "Follow Theme",
            "Accent",
            "Background",
            "Corner Radius",
            "Edit Layout",
        ],
        "Custom UI owns the pause screen and the hotbar, nothing else"
    );
    for foreign in [
        "Accent Bar",
        "Accent Bar Width",
        "Animation",
        "Border Opacity",
        "Border Width",
        "Card Opacity",
        "Comet Speed",
        "Comet Tail",
        "Comets",
        "Editor Dim",
        "Glow Pulse",
        "Glow Strength",
        "Gradient Strength",
        "Grid Size",
        "Grid Style",
        "Highlight Hovered",
        "Hover Lift",
        "Lock Widgets",
        "Lowercase",
        "Padding",
        "Panel Gradient",
        "Panel Opacity",
        "Pulse Speed",
        "Reveal Distance",
        "Row Height",
        "Shadow Offset",
        "Shadow Opacity",
        "Show Grid",
        "Show Guides",
        "Show Hint",
        "Show Labels",
        "Show Sizes",
        "Snap Distance",
        "Snap To Edges",
        "Snap To Grid",
        "Stagger",
        "Text",
        "Text Shadow",
        "Text Size",
        "Widget Scale",
    ] {
        assert!(
            data.get_setting(foreign).is_none(),
            "`{foreign}` does not belong in the Custom UI panel"
        );
    }

    assert!(toggle(data, "Pause Menu"));
    assert!(toggle(data, "Pause Animations"));
    assert!(toggle(data, "Hotbar"));
    assert!(toggle(data, "Hotbar Glow"));
    assert!(toggle(data, "Hotbar Numbers"));
    assert!(toggle(data, "Item Name"));
    assert!(!toggle(data, "Follow Theme"));
    assert!(!toggle(data, "Edit Layout"));
    assert_eq!(slider(data, "Pause Speed"), 1.0);
    assert_eq!(slider(data, "Backdrop Dim"), 0.66);
    assert_eq!(slider(data, "Hotbar Scale"), 1.0);
    assert_eq!(slider(data, "Slide Speed"), 1.15);
    assert_eq!(slider(data, "Corner Radius"), 7.0);

    let accent = match data.get_setting("Accent") {
        Some(ModuleSetting::Color { value, .. }) => *value,
        other => panic!("missing accent colour: {other:?}"),
    };
    assert_eq!(accent, ECHO);
    assert!(
        accent[0] > 0.9 && accent[1] > 0.3 && accent[2] > 0.3,
        "the stock accent is a light red, got {accent:?}"
    );
}

#[test]
fn the_pause_menu_follows_the_module_and_its_switch() {
    let mut module = CustomUiModule::new();
    assert!(!pause_menu_on(&module.module), "off with the module");
    module.module.set_enabled(true);
    assert!(pause_menu_on(&module.module), "on with the module");
    if let Some(ModuleSetting::Toggle { value, .. }) = module.module.get_setting_mut("Pause Menu") {
        *value = false;
    }
    assert!(
        !pause_menu_on(&module.module),
        "the switch can keep the custom UI but restore the vanilla pause menu"
    );
}

#[test]
fn the_hotbar_follows_the_module_and_its_switch() {
    let mut module = CustomUiModule::new();
    assert!(!hotbar_on(&module.module), "off with the module");
    module.module.set_enabled(true);
    assert!(hotbar_on(&module.module), "on with the module");
    if let Some(ModuleSetting::Toggle { value, .. }) = module.module.get_setting_mut("Hotbar") {
        *value = false;
    }
    assert!(
        !hotbar_on(&module.module),
        "the switch hands the bar back to vanilla without disabling the pause screen"
    );
    assert!(pause_menu_on(&module.module));
}

#[test]
fn every_hotbar_setting_reaches_its_snapshot() {
    let mut module = CustomUiModule::new();
    let data = &mut module.module;
    if let Some(ModuleSetting::Slider { value, .. }) = data.get_setting_mut("Hotbar Scale") {
        *value = 1.35;
    }
    if let Some(ModuleSetting::Slider { value, .. }) = data.get_setting_mut("Slide Speed") {
        *value = 2.0;
    }
    if let Some(ModuleSetting::Toggle { value, .. }) = data.get_setting_mut("Hotbar Glow") {
        *value = false;
    }
    if let Some(ModuleSetting::Toggle { value, .. }) = data.get_setting_mut("Hotbar Numbers") {
        *value = false;
    }
    if let Some(ModuleSetting::Toggle { value, .. }) = data.get_setting_mut("Item Name") {
        *value = false;
    }
    if let Some(ModuleSetting::Slider { value, .. }) = data.get_setting_mut("Corner Radius") {
        *value = 12.0;
    }
    if let Some(ModuleSetting::Color { value, .. }) = data.get_setting_mut("Accent") {
        *value = [0.2, 0.4, 0.6, 1.0];
    }

    let style = HotbarStyle::from_data(data);
    assert_eq!(style.scale, 1.35);
    assert_eq!(style.slide_speed, 2.0);
    assert!(!style.glow);
    assert!(!style.numbers);
    assert!(!style.item_name);
    assert_eq!(style.radius, 12.0);
    assert_eq!(style.accent, Color32::from_rgb(51, 102, 153));

     
    let faster = style.slide_spring();
    let slower = HotbarStyle {
        slide_speed: 0.6,
        ..style
    }
    .slide_spring();
    assert!(faster.stiffness > slower.stiffness);
    assert!(faster.damping > slower.damping);
}

#[test]
fn every_pause_setting_reaches_its_snapshot() {
    let mut module = CustomUiModule::new();
    let data = &mut module.module;
    if let Some(ModuleSetting::Toggle { value, .. }) = data.get_setting_mut("Pause Animations") {
        *value = false;
    }
    if let Some(ModuleSetting::Slider { value, .. }) = data.get_setting_mut("Pause Speed") {
        *value = 2.2;
    }
    if let Some(ModuleSetting::Slider { value, .. }) = data.get_setting_mut("Backdrop Dim") {
        *value = 0.3;
    }
    if let Some(ModuleSetting::Toggle { value, .. }) = data.get_setting_mut("Embers") {
        *value = false;
    }
    if let Some(ModuleSetting::Slider { value, .. }) = data.get_setting_mut("Ember Count") {
        *value = 64.0;
    }
    if let Some(ModuleSetting::Toggle { value, .. }) = data.get_setting_mut("Follow Theme") {
        *value = true;
    }

    let style = PauseStyle::from_data(data);
    assert!(!style.animations);
    assert_eq!(style.speed, 2.2);
    assert_eq!(style.dim, 0.3);
    assert!(!style.embers);
    assert_eq!(style.ember_count, 64);
    assert_eq!(style.accent, theme::accent());
}

#[test]
fn following_the_theme_adopts_the_menu_accent_on_both_surfaces() {
    let mut module = CustomUiModule::new();
    let data = &mut module.module;
    assert!(!toggle(data, "Follow Theme"));
    assert_eq!(palette_accent(data), Color32::from_rgb(255, 92, 107));

    if let Some(ModuleSetting::Toggle { value, .. }) = data.get_setting_mut("Follow Theme") {
        *value = true;
    }
    assert_eq!(palette_accent(data), theme::accent());
    assert_eq!(HotbarStyle::from_data(data).accent, theme::accent());
    assert_eq!(PauseStyle::from_data(data).accent, theme::accent());
}

#[test]
fn the_reveal_cascades_each_widget_in_order() {
    let style = UiStyle {
        animation: 0,
        animation_speed: 1.0,
        reveal_distance: 20.0,
        stagger: 0.06,
        ..UiStyle::default()
    };
    let count = 12;
    assert_eq!(style.reveal_at(0.0, 0, count).alpha, 0.0);
    assert_eq!(style.reveal_at(0.0, 11, count).alpha, 0.0);
    let first = style.reveal_at(0.4, 0, count);
    let last = style.reveal_at(0.4, 11, count);
    assert!(first.alpha > last.alpha, "earlier widgets reveal first");
    assert!(first.offset.y >= 0.0, "the default slides up into place");

    let settled = style.reveal_at(1.0, 11, count);
    assert_eq!(settled.alpha, 1.0);
    assert_eq!(settled.offset, Vec2::ZERO);

    let side = UiStyle {
        animation: 2,
        ..style
    };
    assert!(side.reveal_at(0.05, 0, count).offset.x > 0.0);
    assert_eq!(side.reveal_at(0.05, 0, count).offset.y, 0.0);

    let fade = UiStyle {
        animation: 3,
        ..style
    };
    assert_eq!(fade.reveal_at(0.05, 0, count).offset, Vec2::ZERO);
}

#[test]
fn labels_follow_the_lowercase_switch_and_rows_never_clip_text() {
    let mut style = UiStyle {
        lowercase: true,
        text_size: 16.0,
        row_height: 14.0,
        ..UiStyle::default()
    };
    assert_eq!(style.label("Armor Status"), "armor status");
    assert_eq!(style.line_height(), 20.0);
    style.lowercase = false;
    assert_eq!(style.label("Armor Status"), "Armor Status");
}
