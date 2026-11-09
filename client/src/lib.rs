 
 
#![allow(dead_code)]

extern crate jni;
mod addons;
mod camera_bridge;
mod config;
mod friends;
mod graphic;
mod jvmti;
mod macros;
mod mapping;
mod module;
mod net;
mod state;

 
 
#[allow(clippy::all, clippy::pedantic)]
mod gl {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

use crate::graphic::hook::{install_hooks, uninstall_hooks};
use crate::module::combat::aimbot::AimbotModule;
use crate::module::combat::anchor_macro::AnchorMacroModule;
use crate::module::combat::auto_bow::AutoBowModule;
use crate::module::combat::auto_mace::AutoMaceModule;
use crate::module::combat::auto_milk::AutoMilkModule;
use crate::module::combat::auto_pearl::AutoPearlModule;
use crate::module::combat::auto_shield::AutoShieldModule;
use crate::module::combat::auto_sword::AutoSwordModule;
use crate::module::combat::auto_weapon::AutoWeaponModule;
use crate::module::combat::autoclicker::AutoClickerModule;
use crate::module::combat::autocrystal::AutoCrystalModule;
use crate::module::combat::crystal_macro::CrystalMacroModule;
use crate::module::combat::crystal_optimizer::CrystalOptimizerModule;
use crate::module::combat::crystal_spam::CrystalSpamModule;
use crate::module::combat::hit_speed::HitSpeedModule;
use crate::module::combat::hitselect::HitSelectModule;
use crate::module::combat::hitswap::HitSwapModule;
use crate::module::combat::killaura::KillAuraModule;
use crate::module::combat::mobaura::MobAuraModule;
use crate::module::combat::no_miss_cooldown::NoMissCooldownModule;
use crate::module::combat::reach::ReachModule;
use crate::module::combat::silent_assist::SilentAssistModule;
use crate::module::combat::silent_aura::SilentAuraModule;
use crate::module::combat::silent_mace::SilentMaceModule;
use crate::module::combat::triggerbot::TriggerBotModule;
use crate::module::combat::velocity::VelocityModule;
use crate::module::combat::wtap::WTapModule;
use crate::module::misc::antiafk::AntiAfkModule;
use crate::module::misc::auto_log::AutoLogModule;
use crate::module::misc::auto_save::AutoSaveModule;
use crate::module::misc::auto_walk::AutoWalkModule;
use crate::module::misc::autoserverjoin::AutoServerJoinModule;
use crate::module::misc::entity_scale::EntityScaleModule;
use crate::module::misc::freelook::FreelookModule;
use crate::module::misc::no_fov_effects::NoFovEffectsModule;
use crate::module::misc::panic::PanicModule;
use crate::module::misc::zoom::ZoomModule;
use crate::module::movement::autojump::AutoJumpModule;
use crate::module::movement::fakelag::FakeLagModule;
use crate::module::movement::fly::FlyModule;
use crate::module::movement::inventory_move::InventoryMoveModule;
use crate::module::movement::jump_reset::JumpResetModule;
use crate::module::movement::no_jump_delay::NoJumpDelayModule;
use crate::module::movement::no_rotate::NoRotateModule;
use crate::module::movement::nofall::NoFallModule;
use crate::module::movement::noslowdown::NoSlowdownModule;
use crate::module::movement::parkour::ParkourModule;
use crate::module::movement::safewalk::SafeWalkModule;
use crate::module::movement::sneak::SneakModule;
use crate::module::movement::speed::SpeedModule;
use crate::module::movement::sprint::SprintModule;
use crate::module::movement::strafe::StrafeModule;
use crate::module::performance::biome_blend_cap::BiomeBlendCapModule;
use crate::module::performance::entity_distance_cap::EntityDistanceCapModule;
use crate::module::performance::entity_shadows_off::EntityShadowsOffModule;
use crate::module::performance::framerate_cap::FramerateCapModule;
use crate::module::performance::menu_blur_off::MenuBlurOffModule;
use crate::module::performance::menu_esp_pause::MenuEspPauseModule;
use crate::module::performance::render_distance_cap::RenderDistanceCapModule;
use crate::module::performance::simulation_distance_cap::SimulationDistanceCapModule;
use crate::module::performance::smooth_lighting_off::SmoothLightingOffModule;
use crate::module::performance::vsync_off::VsyncOffModule;
use crate::module::player::armorswitch::ArmorSwitchModule;
use crate::module::player::auto_gapple::AutoGappleModule;
use crate::module::player::autoarmor::AutoArmorModule;
use crate::module::player::autototem::AutoTotemModule;
use crate::module::player::cheststealer::ChestStealerModule;
use crate::module::player::freecam::FreecamModule;
use crate::module::player::invcleaner::InvCleanerModule;
use crate::module::player::inventory_manager::InventoryManagerModule;
use crate::module::player::middleclick::MiddleClickModule;
use crate::module::player::refill::RefillModule;
use crate::module::player::totempanic::TotemPanicModule;
use crate::module::render::atmosphere::AtmosphereModule;
use crate::module::render::aura::AuraModule;
use crate::module::render::block_esp::BlockEspModule;
use crate::module::render::block_outline::BlockOutlineModule;
use crate::module::render::box_esp::BoxEspModule;
use crate::module::render::break_visualizer::BreakVisualizerModule;
use crate::module::render::chams::ChamsModule;
use crate::module::render::chest_esp::ChestEspModule;
use crate::module::render::crosshair::CrosshairModule;
use crate::module::render::crown::CrownModule;
use crate::module::render::custom_sky::CustomSkyModule;
use crate::module::render::death_effect::DeathEffectModule;
use crate::module::render::esp_modules::{item_esp, projectiles, spawner_finder};
use crate::module::render::explosion::ExplosionModule;
use crate::module::render::floating_cubes::FloatingCubesModule;
use crate::module::render::footprints::FootprintsModule;
use crate::module::render::fov::FovModule;
use crate::module::render::fullbright::FullbrightModule;
use crate::module::render::hand_aura::HandAuraModule;
use crate::module::render::hand_outline::HandOutlineModule;
use crate::module::render::haze::HazeModule;
use crate::module::render::hit_effect::HitEffectModule;
use crate::module::render::hologram_chams::HologramChamsModule;
use crate::module::render::infered_tag::InferedTagModule;
use crate::module::render::invis_fire::InvisFireModule;
use crate::module::render::invisible_shield::InvisibleShieldModule;
use crate::module::render::item_chams::ItemChamsModule;
use crate::module::render::item_scale::ItemScaleModule;
use crate::module::render::jump_circles::JumpCirclesModule;
use crate::module::render::legit::{
    armor_status, clock, compass, coords, fps, key_strokes, lyrics, menu_hud, module_list, ping,
    potion_status, radar, session, speedometer, spotify_now_playing, status, top_bar,
    totem_counter,
};
use crate::module::render::mob_esp::MobEspModule;
use crate::module::render::motion_blur::MotionBlurModule;
use crate::module::render::nametags::NametagsModule;
use crate::module::render::no_hurtcam::NoHurtCamModule;
use crate::module::render::no_particles::NoParticlesModule;
use crate::module::render::no_weather::NoWeatherModule;
use crate::module::render::player_esp::PlayerEspModule;
use crate::module::render::reflection::ReflectionModule;
use crate::module::render::saturation::SaturationModule;
use crate::module::render::skeleton_esp::SkeletonEspModule;
use crate::module::render::swing_echo::SwingEchoModule;
use crate::module::render::totem_pop::TotemPopModule;
use crate::module::render::tracers::TracersModule;
use crate::module::render::trail::TrailModule;
use crate::module::render::trajectories::TrajectoriesModule;
use crate::module::render::xray::XrayModule;
use crate::module::world::auto_fish::AutoFishModule;
use crate::module::world::autoeat::AutoEatModule;
use crate::module::world::autotool::AutoToolModule;
use crate::module::world::chunk_boost::ChunkBoostModule;
use crate::module::world::clutch::ClutchModule;
use crate::module::world::fast_scaffold::FastScaffoldModule;
use crate::module::world::fastplace::FastPlaceModule;
use crate::module::world::nuker::NukerModule;
use crate::module::world::scaffold::ScaffoldModule;
use crate::state::{client, init};
use log::{error, info, LevelFilter};
use simplelog::{Config, WriteLogger};
use std::fs::File;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

 
pub(crate) static RUNNING: AtomicBool = AtomicBool::new(false);

 
#[no_mangle]
pub extern "C" fn initialize_client() {
     
    if RUNNING.swap(true, Ordering::SeqCst) {
        info!("Client already initialized");
        return;
    }

     
     
     
     
     
    let log_path = config::base_dir().join("infered.log");
    match File::create(&log_path) {
        Ok(file) => match WriteLogger::init(
            if cfg!(any(debug_assertions, feature = "devtest")) {
                LevelFilter::Debug
            } else {
                LevelFilter::Warn
            },
            Config::default(),
            file,
        ) {
            Ok(_) => info!("Logger initialized"),
            Err(e) => {
                info!("Using the already-installed logger (infered.log unavailable): {e}")
            }
        },
        Err(e) => info!("Logger file unavailable ({e:?}); falling back to the installed logger"),
    }

     
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        error!("Infered panicked! Restoring input/render hooks…");
        cleanup_client();
        default_hook(panic_info);
    }));

    thread::spawn(|| {
        info!("Starting Infered…");

         
         
         
        if let Err(e) = init() {
            error!("Client initialization failed: {e}");
            return;
        }
        register_modules();
        if let Err(e) = jvmti::install() {
            error!("Failed to install Camera.setup JVMTI hook: {e}");
        }
         
         
        addons::load_all();
         
         
        config::load();
         
         
        macros::init();
        config::write_catalog();
        if let Err(e) = install_hooks() {
            error!("Failed to install hooks: {e}");
        }
        info!("Infered started");
    });
}

 
#[no_mangle]
pub extern "C" fn cleanup_client() {
    info!("Client cleanup in progress…");
    RUNNING.store(false, Ordering::SeqCst);
    crate::jvmti::uninstall();
    if let Err(error) = crate::graphic::smoke::stop() {
        log::warn!("[Smoke] cleanup failed: {error}");
    }
    crate::camera_bridge::uninstall();
    uninstall_hooks();
    crate::graphic::input::cleanup();
     
     
     
     
     
     
    if let Some(client) = crate::state::try_client() {
        client.modules.set_active(false);
    }
     
     
    crate::macros::shutdown();
     
     
     
    crate::graphic::esp::shutdown_skin_loaders();
    crate::module::render::death_effect::shutdown_hologram_loaders();
    crate::graphic::player_card::shutdown();
     
     
     
    addons::shutdown();
     
     
    crate::graphic::title_logo::release_splash();
    crate::state::teardown();
    info!("Client cleanup completed");
}

 
fn register_modules() {
    let modules = &client().modules;
    modules.register(FlyModule::new());
    modules.register(NoFallModule::new());
    modules.register(NoJumpDelayModule::new());
    modules.register(NoRotateModule::new());
    modules.register(FakeLagModule::new());
    modules.register(StrafeModule::new());
    modules.register(KillAuraModule::new());
    modules.register(MobAuraModule::new());
    modules.register(SilentAuraModule::new());
    modules.register(AimbotModule::new());
    modules.register(AutoCrystalModule::new());
    modules.register(AuraModule::new());
    modules.register(crate::module::render::cosmetics::CosmeticModule::cape());
    modules.register(crate::module::render::cosmetics::CosmeticModule::wings());
    modules.register(ChamsModule::new());
    modules.register(ItemChamsModule::new());
    modules.register(VelocityModule::new());
    modules.register(PlayerEspModule::new());
    modules.register(MobEspModule::new());
    modules.register(BoxEspModule::new());
    modules.register(ChestEspModule::new());
    modules.register(BlockEspModule::new());
    modules.register(BlockOutlineModule::new());
    modules.register(BreakVisualizerModule::new());
    modules.register(NoHurtCamModule::new());
    modules.register(ExplosionModule::new());
    modules.register(ItemScaleModule::new());
    modules.register(CustomSkyModule::new());
    modules.register(AtmosphereModule::new());
    modules.register(HazeModule::new());
    modules.register(ReflectionModule::new());
    modules.register(CrownModule::new());
     
     
     
     
     
     
     
     
     
    modules.register(CrosshairModule::new());
    modules.register(crate::module::render::title_logo::TitleLogoModule::new());
    modules.register(InvisibleShieldModule::new());
    modules.register(SwingEchoModule::new());
    modules.register(HandOutlineModule::new());
    modules.register(HandAuraModule::new());
    modules.register(InvisFireModule::new());
    modules.register(NoParticlesModule::new());
    modules.register(NoWeatherModule::new());
    modules.register(TotemPopModule::new());
    modules.register(XrayModule::new());
    modules.register(TracersModule::new());
    modules.register(MotionBlurModule::new());
    modules.register(SaturationModule::new());
    modules.register(FullbrightModule::new());
     
     
     
     
     
     
    modules.register(FovModule::new());
    modules.register(AntiAfkModule::new());
    modules.register(SprintModule::new());
    modules.register(SneakModule::new());
    modules.register(NametagsModule::new());
    modules.register(crate::module::render::target_info::TargetInfoModule::new());
    modules.register(InferedTagModule::new());
    modules.register(SpeedModule::new());
    modules.register(NoSlowdownModule::new());
    modules.register(SafeWalkModule::new());
    modules.register(InventoryMoveModule::new());
    modules.register(AutoJumpModule::new());
    modules.register(ParkourModule::new());
    modules.register(JumpResetModule::new());
    modules.register(AutoTotemModule::new());
    modules.register(AutoGappleModule::new());
    modules.register(TotemPanicModule::new());
    modules.register(AutoServerJoinModule::new());
    modules.register(TrailModule::new());
    modules.register(FootprintsModule::new());
    modules.register(JumpCirclesModule::new());
    modules.register(FloatingCubesModule::new());
    modules.register(TrajectoriesModule::new());
    modules.register(HitEffectModule::new());
    modules.register(DeathEffectModule::new());
    modules.register(HologramChamsModule::new());
    modules.register(SkeletonEspModule::new());
    modules.register(ChestStealerModule::new());
    modules.register(AutoArmorModule::new());
    modules.register(NukerModule::new());
    modules.register(ScaffoldModule::new());
    modules.register(FastScaffoldModule::new());
    modules.register(ClutchModule::new());
    modules.register(FreecamModule::new());
    modules.register(AutoClickerModule::new());
    modules.register(TriggerBotModule::new());
    modules.register(AutoWeaponModule::new());
    modules.register(HitSwapModule::new());
    modules.register(AutoMaceModule::new());
    modules.register(SilentMaceModule::new());
    modules.register(SilentAssistModule::new());
    modules.register(AutoShieldModule::new());
    modules.register(AutoMilkModule::new());
    modules.register(AutoSwordModule::new());
    modules.register(AutoPearlModule::new());
    modules.register(AutoBowModule::new());
    for module in crate::module::combat::practical::modules() {
        modules.register(module);
    }
    modules.register(CrystalMacroModule::new());
    modules.register(CrystalSpamModule::new());
    modules.register(AnchorMacroModule::new());
    modules.register(CrystalOptimizerModule::new());
    modules.register(NoMissCooldownModule::new());
    modules.register(HitSpeedModule::new());
    modules.register(ReachModule::new());
    modules.register(HitSelectModule::new());
    modules.register(WTapModule::new());
    modules.register(ChunkBoostModule::new());
    modules.register(RenderDistanceCapModule::new());
    modules.register(EntityDistanceCapModule::new());
    modules.register(EntityShadowsOffModule::new());
    modules.register(SmoothLightingOffModule::new());
    modules.register(BiomeBlendCapModule::new());
    modules.register(FramerateCapModule::new());
    modules.register(VsyncOffModule::new());
    modules.register(SimulationDistanceCapModule::new());
    modules.register(MenuBlurOffModule::new());
    modules.register(MenuEspPauseModule::new());
    for module in crate::module::performance::client_optimizations::modules() {
        modules.register(module);
    }
    modules.register(FastPlaceModule::new());
    modules.register(InvCleanerModule::new());
    modules.register(RefillModule::new());
    modules.register(MiddleClickModule::new());
    modules.register(PanicModule::new());
    modules.register(NoFovEffectsModule::new());
    for module in crate::module::misc::option_utilities::modules() {
        modules.register(module);
    }
    modules.register(crate::module::misc::durability::DurabilityModule::new(false));
    modules.register(crate::module::misc::durability::DurabilityModule::new(true));
    modules.register(AutoLogModule::new());
    modules.register(AutoSaveModule::new());
    modules.register(AutoWalkModule::new());
    modules.register(EntityScaleModule::new());
    modules.register(ZoomModule::new());
    modules.register(item_esp());
    modules.register(projectiles());
    modules.register(spawner_finder());
    modules.register(FreelookModule::new());
    modules.register(AutoToolModule::new());
    modules.register(AutoEatModule::new());
    modules.register(AutoFishModule::new());
    modules.register(crate::macros::module::MacrosModule::new());
    modules.register(ArmorSwitchModule::new());
    modules.register(InventoryManagerModule::new());
    modules.register(coords());
    modules.register(fps());
    modules.register(compass());
    modules.register(key_strokes());
    modules.register(clock());
    modules.register(speedometer());
    modules.register(session());
    modules.register(ping());
    modules.register(potion_status());
    modules.register(armor_status());
    modules.register(status());
    modules.register(totem_counter());
    modules.register(radar());
    modules.register(menu_hud());
    modules.register(module_list());
    modules.register(top_bar());
    modules.register(spotify_now_playing());
    modules.register(lyrics());
     
     
     
    #[cfg(feature = "devtest")]
    {
        modules.register(crate::module::dev::dev_mode::DevModeModule::new());
        modules.register(crate::module::dev::module_tester::ModuleTesterModule::new());
    }
}
