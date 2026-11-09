 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::theme;
use crate::module::{ModuleId, ModuleSetting};
use egui::{Context, Id, LayerId, RichText};
use std::borrow::Cow;
use std::time::Duration;

 
pub const TOOLTIP_DELAY_SECONDS: f64 = 1.0;

 
 
 
 
pub fn delayed_tooltip(
    ctx: &Context,
    id: Id,
    layer: LayerId,
    hovered: bool,
    title: &str,
    body: &str,
) {
    let since_id = id.with("hover_since");
    if !hovered {
        ctx.data_mut(|data| data.remove::<f64>(since_id));
        return;
    }

    let now = ctx.input(|input| input.time);
    let since = match ctx.data(|data| data.get_temp::<f64>(since_id)) {
        Some(since) => since,
        None => {
            ctx.data_mut(|data| data.insert_temp(since_id, now));
            now
        }
    };

    if !tooltip_due(since, now) {
         
         
        ctx.request_repaint_after(Duration::from_millis(50));
        return;
    }

    let body = if body.trim().is_empty() {
        "No description available."
    } else {
        body
    };
    egui::show_tooltip_at_pointer(ctx, layer, id.with("popup"), |ui| {
        ui.set_max_width(250.0);
        ui.label(RichText::new(title).size(12.0).strong().color(theme::TEXT));
        ui.add_space(2.0);
        ui.label(RichText::new(body).size(11.0).color(theme::TEXT_DIM));
    });
}

 
fn tooltip_due(since: f64, now: f64) -> bool {
    now - since >= TOOLTIP_DELAY_SECONDS - 1e-6
}

 
 
pub fn setting_help(module: ModuleId, name: &str, setting: &ModuleSetting) -> Cow<'static, str> {
    if let Some(text) = specific_help(module, name) {
        return Cow::Borrowed(text);
    }
    if let Some(text) = common_help(name) {
        return Cow::Borrowed(text);
    }
    Cow::Owned(fallback_help(name, setting))
}

 
fn specific_help(module: ModuleId, name: &str) -> Option<&'static str> {
    match (module, name) {
         
        (ModuleId::TotemPanic, "Health") => Some("Health level at which the totem swap triggers."),
        (ModuleId::AutoLog, "Health") => Some("Health level at which the client presses Panic."),
        (ModuleId::AutoGapple, "Health") => {
            Some("Health at or below which a golden apple is eaten.")
        }
        (ModuleId::AutoGapple, "Item") => Some("Which golden apple variants may be eaten."),
        (ModuleId::AutoGapple, "Cooldown") => {
            Some("Wait after an apple before another may be eaten, in seconds.")
        }
        (ModuleId::NoJumpDelay, "Delay") => {
            Some("Jump cooldown left in place, in ticks; 0 jumps as soon as you land.")
        }
        (ModuleId::NoWeather, "Rain") => Some("Hide rain visually while the module is on."),
        (ModuleId::NoWeather, "Thunder") => {
            Some("Hide thunder and lightning flashes while the module is on.")
        }
        (ModuleId::AutoFish, "Auto Cast") => Some("Cast automatically whenever no hook is out."),
        (ModuleId::AutoFish, "Reel Delay") => {
            Some("Wait after a bite before reeling, in milliseconds.")
        }
        (ModuleId::AutoFish, "Cast Delay") => {
            Some("Wait after a reel before casting again, in milliseconds.")
        }
        (ModuleId::AutoLog, "Players") => Some("Only panic while another player is close by."),
        (ModuleId::AutoLog, "Range") => Some("How close another player must be to count."),
        (ModuleId::AutoLog, "Delay") => {
            Some("Grace period before Panic fires, so a quick heal cancels it.")
        }
        (ModuleId::NoFovEffects, "Scale") => {
            Some("FOV effect strength that replaces the vanilla one; 0 disables it.")
        }
        (ModuleId::AutoSave, "Interval") => Some("How often the config is saved, in seconds."),
        (ModuleId::AutoSave, "Save On Leave") => Some("Save once more when you leave a world."),
        (ModuleId::Zoom, "Zoom FOV") => Some("Field of view while the zoom key is held."),
        (ModuleId::Zoom, "Key") => Some("Key held down to zoom."),
        (ModuleId::AutoWalk, "Direction") => Some("Direction AutoWalk keeps walking."),
        (ModuleId::AutoWalk, "Sprint") => Some("Also hold sprint while walking forward."),
        (ModuleId::AutoWalk, "Pause In Menus") => {
            Some("Release the movement keys while a screen is open.")
        }
         
         
        (ModuleId::Chams, "Style") => Some(
            "Every look: Neon Trail, Solid, Scanline, Ember and Video draw with the classical \
             material, while Glow, Blur, Galaxy, Prism, Neon, Scan, Liquid Glass, Outline, \
             Contour, Cel and Pulse draw through Warped's outline pass.",
        ),
        (ModuleId::Chams, "Glow Radius") => Some("How far the light reaches outside the body."),
        (ModuleId::Chams, "Fill Opacity") => Some("How solid the body itself is drawn."),
        (ModuleId::Chams, "Outline Width") => Some("Thickness of the body's rim."),
        (ModuleId::Chams, "Blur") => Some("How soft every edge of the look is."),
        (ModuleId::Chams, "Tint Amount") => Some("How much of the module's Color the body takes."),
        (ModuleId::Chams, "Hairline") => Some("Draw the thin light line just inside the rim."),
        (ModuleId::Chams, "Thickness") => {
            Some("How far the glass and the inner glow reach in from the rim.")
        }
        (ModuleId::Chams, "Refraction") => {
            Some("How far Liquid Glass bends the scenery behind the body.")
        }
        (ModuleId::Chams, "Dispersion") => Some("RGB split of that refraction."),
         
        (ModuleId::SkeletonEsp, "Skull") => {
            Some("Draw the skull: a cranium with a jaw and brow, over the neck.")
        }
        (ModuleId::SkeletonEsp, "Ribcage") => Some(
            "Draw the ribcage: ribs from the spine round to the sternum, plus the shoulder blades.",
        ),
        (ModuleId::SkeletonEsp, "Rib Pairs") => Some("How many pairs of ribs the cage has."),
        (ModuleId::SkeletonEsp, "Spine") => {
            Some("Draw the spine: a column of vertebrae from the waist to the skull.")
        }
        (ModuleId::SkeletonEsp, "Spine Segments") => Some("How many vertebrae are in the spine."),
        (ModuleId::SkeletonEsp, "Pelvis") => {
            Some("Draw the pelvis: two hip wings and the pubis, where the legs meet the spine.")
        }
        (ModuleId::SkeletonEsp, "Clavicles") => {
            Some("Draw the collarbones from the sternum out to each shoulder.")
        }
        (ModuleId::SkeletonEsp, "Arms") => {
            Some("Draw the arms: humerus, radius and ulna, bent at the elbow.")
        }
        (ModuleId::SkeletonEsp, "Legs") => {
            Some("Draw the legs: femur, tibia and fibula, bent at the knee.")
        }
        (ModuleId::SkeletonEsp, "Hands") => Some("Draw a palm and fingers at each wrist."),
        (ModuleId::SkeletonEsp, "Feet") => Some("Draw a foot pointing forward at each ankle."),
        (ModuleId::SkeletonEsp, "Joints") => {
            Some("Draw a ball at every articulation, so the bones read as one skeleton.")
        }
        (ModuleId::SkeletonEsp, "Detail") => {
            Some("How many sides each bone is drawn with. Low is the same skeleton, cheaper.")
        }
        (ModuleId::SkeletonEsp, "Thickness") => {
            Some("Base thickness of a bone; every part of the rig scales from it.")
        }
        (ModuleId::SkeletonEsp, "Glow") => Some("Strength of the rim light on the bones."),
        (ModuleId::SkeletonEsp, "ThroughWalls") => {
            Some("Draw the skeleton through blocks, not just when the player is in view.")
        }
        (ModuleId::Chams, "Frost") => Some("Frosted specks inside the body."),
        (ModuleId::Chams, "Glass Tint") => Some("White wash Liquid Glass puts over the body."),
        (ModuleId::Chams, "Cel Opacity") => Some("How solid Cel's flat tones are."),
        (ModuleId::Chams, "Reactivity") => Some("How hard the beat moves Pulse's light."),
        (ModuleId::Chams, "Idle Beats") => Some("Keep the beat going when no track is playing."),
        (ModuleId::Chams, "Audio Sensitivity") => Some("How strongly the music drives the beat."),
         
        (ModuleId::Crown, "Style") => Some("Inferno, Halo or Spiked — the crown's shape family."),
        (ModuleId::Crown, "Core Color") => {
            Some("White-hot colour of the flame roots and the band's rim light.")
        }
        (ModuleId::Crown, "Glow") => Some("Bloom behind the crown and around each flame."),
        (ModuleId::Crown, "Glow Strength") => Some("How bright that bloom is."),
        (ModuleId::Crown, "Size") => Some("Scales the band, points, flames and embers together."),
        (ModuleId::Crown, "Opacity") => Some("Overall opacity of the drawn crown."),
        (ModuleId::Crown, "Range") => Some("How far away a player still gets a crown."),
        (ModuleId::Crown, "Points") => Some("How many points the band has."),
        (ModuleId::Crown, "Hover") => {
            Some("How far the crown floats above the head; 0 rests it on the head.")
        }
        (ModuleId::Crown, "Flicker") => Some("How much the flames lengthen and dim as they dance."),
        (ModuleId::Crown, "Rainbow") => {
            Some("Cycle the flame and gem colours through the spectrum.")
        }
        (ModuleId::Crown, "Show Self") => Some("Your own crown, visible in third person."),
        (ModuleId::Crown, "Show Others") => Some("Crowns on other players."),
        (ModuleId::Crown, "Embers") => Some("Number of sparks rising off the crown."),
         
        (ModuleId::CustomCrosshair, "Cooldown Style") => Some(
            "Ring fills around the reticle and flashes when the hit is ready; \
             Bar is the classic strip under it.",
        ),
        (ModuleId::CustomCrosshair, "Dot") => {
            Some("Draw the diamond in the middle of the reticle.")
        }
        (ModuleId::CustomCrosshair, "Glow") => {
            Some("Soft halo around the reticle and the charge arc.")
        }
        (ModuleId::CustomCrosshair, "Outline") => {
            Some("Dark edge behind the reticle and a light rim on the ring.")
        }
         
        (ModuleId::TargetInfo, "Physical") => Some(
            "Draw health on the target's left, items on their right and the rest \
             beside them, in the world, so the readout turns with the target.",
        ),
        (ModuleId::EntityScale, "Scale") => {
            Some("Render size of distant entities; 1.0 keeps them full size.")
        }
         
        (ModuleId::Clock, "Style") => {
            Some("Analog: a drawn watch face with hands. Digital: a card with the time set large.")
        }
        (ModuleId::Clock, "24 Hour") => Some("Show the clock in 24-hour time."),
        (ModuleId::Clock, "Seconds") => {
            Some("Show seconds — the second hand, or the seconds after the time.")
        }
        (ModuleId::Clock, "Show Date") => Some("Show today's date under the time."),
        (ModuleId::Clock, "Timezone") => {
            Some("Local uses this machine's own time zone, daylight saving included; UTC does not.")
        }
        (ModuleId::Clock, "Smooth Seconds") => {
            Some("Sweep the second hand between the seconds instead of ticking it.")
        }
        (ModuleId::Clock, "Scale") => Some("Size of the clock face and its readout."),
        (ModuleId::Nametags, "Health") => Some("Show each player's health on their name tag."),
        (ModuleId::PlayerEsp, "Style") => {
            Some("Brackets: hugging corner marks. Rounded: a glowing frame with a moving comet.")
        }
        (ModuleId::MobEsp, "Style") => {
            Some("Brackets: hugging corner marks. Rounded: a glowing frame with a moving comet.")
        }
        (ModuleId::PlayerEsp, "Animate") => Some("Breathe the box and run the comet around it."),
        (ModuleId::MobEsp, "Animate") => Some("Breathe the box and run the comet around it."),
        (ModuleId::PlayerEsp, "Glow") => Some("Soft halo behind the box."),
        (ModuleId::MobEsp, "Glow") => Some("Soft halo behind the box."),
        (ModuleId::PlayerEsp, "Health") => Some("Show player health in the ESP."),
        (ModuleId::MobEsp, "Health") => Some("Show mob health in the ESP."),
        (ModuleId::PlayerEsp, "Name") => Some("Show each player's name above the box."),
        (ModuleId::MobEsp, "Name") => Some("Show each mob's name above the box."),
        (ModuleId::ItemEsp, "Name") => Some("Show each dropped item's name."),
        (ModuleId::Projectiles, "Name") => Some("Show the projectile type above the box."),

         
        (ModuleId::Chams, "Armor") => Some("Apply chams to the armor pieces players wear."),
        (ModuleId::Chams, "Video File") => Some(
            "The clip the Video look paints players with. Upload one below, or drop \
             a gif, webp, mp4, webm or a folder of frames into the chams_videos \
             folder next to the config.",
        ),
        (ModuleId::Chams, "Frame Rate") => {
            Some("Playback rate for a frame folder and for a still picture.")
        }

         
        (ModuleId::TriggerBot, "Mobs") => Some("Allow TriggerBot to fire on mobs."),
        (ModuleId::TriggerBot, "Players") => Some("Allow TriggerBot to fire on players."),
        (ModuleId::DeathEffect, "Mobs") => Some("Play the death effect for mobs."),
        (ModuleId::DeathEffect, "Players") => Some("Play the death effect for players."),
        (ModuleId::Tracers, "Mobs") => Some("Draw tracers to mobs."),
        (ModuleId::Tracers, "Players") => Some("Draw tracers to players."),

         
        (ModuleId::AntiAfk, "Action") => Some("Action performed every interval."),
        (ModuleId::AntiAfk, "Randomize") => {
            Some("Vary the interval so the action rhythm does not look mechanical.")
        }
        (ModuleId::AntiAfk, "Jitter") => {
            Some("How much the interval may vary, as a percentage either way.")
        }
        (ModuleId::MiddleClick, "Action") => Some("Action performed by the middle-click bind."),
        (ModuleId::Sneak, "Mode") => Some("How sneak is applied: hold, pulse, or toggle."),
        (ModuleId::ArmorSwitch, "Set") => Some("Armor set to equip."),

         
        (ModuleId::Aimbot, "Speed") => Some("How quickly the aim turns toward the target."),
        (ModuleId::Aimbot, "FOV") => Some("Field of view the aim searches within."),
        (ModuleId::Fov, "FOV") => Some("Field of view applied to the game camera."),
        (ModuleId::Aura, "Speed") => Some("How fast each line crawls, in blocks per second."),
        (ModuleId::Aura, "Range") => Some("Distance around you where new lines appear."),
        (ModuleId::Fly, "Speed") => Some("Flight speed."),
        (ModuleId::Freecam, "Speed") => Some("How fast the detached camera moves."),
        (ModuleId::ChunkBoost, "Speed") => Some("Chunk loading speed multiplier."),
        (ModuleId::EntityDistanceCap, "Scale") => {
            Some("Distance scale applied to entity rendering.")
        }

         
        (ModuleId::Nametags, "Range") => Some("Maximum distance a name tag is drawn."),
        (ModuleId::Reach, "Distance") => Some("Extra attack distance beyond vanilla reach."),
        (ModuleId::Chams, "Outline Glow") => {
            Some("Enable the outline and glow pass for the selected Chams style.")
        }
        (ModuleId::SilentMace, "Min Fall Distance") => {
            Some("How far you must descend before Silent Mace can attack.")
        }
        (ModuleId::SilentMace, "Target Range") => {
            Some("Maximum distance to select a player, capped by the live reach attribute.")
        }
        (ModuleId::SilentMace, "Aim Speed") => {
            Some("How quickly silent aim snaps onto the selected player.")
        }
        (ModuleId::SilentAssist, "Strength") => Some(
            "How far off the crosshair a player can be and still be hit. 0 turns the assist off.",
        ),
        (ModuleId::Freecam, "Distance") => Some("How far the camera may move from the player."),
        (ModuleId::Freelook, "Distance") => Some("How far the freelook camera can orbit."),
        (ModuleId::Nametags, "Distance") => Some("Show each player's distance on their name tag."),
        (ModuleId::PlayerEsp, "Distance") => Some("Show the distance to each player."),
        (ModuleId::MobEsp, "Distance") => Some("Show the distance to each mob."),
        (ModuleId::BoxEsp, "Color") => Some("Colour of the 3D box."),
        (ModuleId::BoxEsp, "Targets") => Some("Which targets Box ESP draws its 3D boxes around."),
        (ModuleId::BoxEsp, "Thickness") => Some("Width of the 3D box's edges."),
        (ModuleId::BoxEsp, "Fill") => Some("Fill the box with a translucent shell."),
        (ModuleId::BoxEsp, "Fill Opacity") => Some("Opacity of the box's translucent faces."),
        (ModuleId::RenderDistanceCap, "Distance") => Some("Maximum render distance."),
        (ModuleId::SimulationDistanceCap, "Distance") => Some("Maximum simulation distance."),

         
        (ModuleId::ItemScale, "Main Hand") => Some("Scale of the held main-hand item."),
        (ModuleId::ItemScale, "Off Hand") => Some("Scale of the held off-hand item."),
        (ModuleId::ItemScale, "Main X" | "Main Y" | "Main Z") => {
            Some("Position offset of the main-hand item.")
        }
        (ModuleId::ItemScale, "Off X" | "Off Y" | "Off Z") => {
            Some("Position offset of the off-hand item.")
        }
        (ModuleId::ItemScale, "Main Pitch" | "Main Yaw" | "Main Roll") => {
            Some("Rotation of the main-hand item.")
        }
        (ModuleId::ItemScale, "Off Pitch" | "Off Yaw" | "Off Roll") => {
            Some("Rotation of the off-hand item.")
        }
        (ModuleId::HandAura, "Main Hand") => Some("Draw the aura around the main hand."),
        (ModuleId::HandAura, "Off Hand") => Some("Draw the aura around the off hand."),
        (ModuleId::InvisibleShield, "Main Hand") => Some("Hide the shield held in the main hand."),
        (ModuleId::InvisibleShield, "Off Hand") => Some("Hide the shield held in the off hand."),

         
        (ModuleId::AutoTotem, "Min Delay") => {
            Some("Fastest reaction time after the off-hand totem is lost.")
        }
        (ModuleId::AutoTotem, "Max Delay") => {
            Some("Slowest reaction time after the off-hand totem is lost.")
        }
        (ModuleId::AutoTotem, "Inventory Delay") => {
            Some("Extra time the inventory stays open before the swap.")
        }

         
        (ModuleId::ModuleList, "Show Logo") => Some("Show the Infered logo above the module list."),
        (ModuleId::ModuleList, "Logo Size") => Some("Height of the Infered logo."),
        (ModuleId::ModuleList, "Logo Opacity") => Some("Opacity of the Infered logo."),
        (ModuleId::ModuleList, "Show Background") => {
            Some("Draw a background behind each module row.")
        }
        (ModuleId::ModuleList, "Background Opacity") => {
            Some("Opacity of the module row backgrounds.")
        }
        (ModuleId::ModuleList, "Show Border") => Some("Draw a border around each module row."),
        (ModuleId::ModuleList, "Text Color") => Some("Color of the module names."),
        (ModuleId::ModuleList, "Text Size") => Some("Font size of the module names."),
        (ModuleId::ModuleList, "Row Height") => Some("Height of each module row."),
        (ModuleId::ModuleList, "Row Spacing") => Some("Gap between module rows."),
        (ModuleId::ModuleList, "Right Margin") => Some("Distance from the right screen edge."),
        (ModuleId::ModuleList, "Top Offset") => Some("Extra space above the module list."),
        (ModuleId::ModuleList, "Sort") => {
            Some("Order of the module list: length, name, or category.")
        }
        (ModuleId::ModuleList, "Accent Bar") => {
            Some("Show the accent bar on the right edge of each row.")
        }
        (ModuleId::ModuleList, "Show Dots") => Some("Show a colored dot before each module name."),
        (ModuleId::ModuleList, "Show Count") => {
            Some("Show the number of enabled modules above the list.")
        }
        (ModuleId::ModuleList, "Lowercase") => Some("Render module names in lowercase."),

         
         
        (ModuleId::CustomUi, "Pause Menu") => {
            Some("Replace Minecraft's pause screen with the Custom UI pause menu.")
        }
        (ModuleId::CustomUi, "Pause Animations") => {
            Some("Play the pause menu's entrance, embers and glow motion.")
        }
        (ModuleId::CustomUi, "Pause Speed") => {
            Some("How quickly the pause card and its buttons settle in.")
        }
        (ModuleId::CustomUi, "Backdrop Dim") => {
            Some("How dark the world gets behind the pause card.")
        }
        (ModuleId::CustomUi, "Embers") => Some("Drift warm embers up the pause screen."),
        (ModuleId::CustomUi, "Ember Count") => Some("How many embers drift behind the card."),
        (ModuleId::CustomUi, "Hotbar") => {
            Some("Draw the Custom UI hotbar over Minecraft's own bar.")
        }
        (ModuleId::CustomUi, "Hotbar Scale") => Some("Size of the hotbar's slot squares."),
        (ModuleId::CustomUi, "Slide Speed") => {
            Some("How quickly the selected-slot box glides to a new slot.")
        }
        (ModuleId::CustomUi, "Hotbar Glow") => {
            Some("Glow the selected slot and run the light along the bar.")
        }
        (ModuleId::CustomUi, "Hotbar Numbers") => Some("Show 1-9 in the corner of each slot."),
        (ModuleId::CustomUi, "Item Name") => {
            Some("Name the held or hovered item above the hotbar.")
        }
        (ModuleId::CustomUi, "Follow Theme") => {
            Some("Use the ClickGUI's accent instead of the light-red one below.")
        }
        (ModuleId::CustomUi, "Accent") => {
            Some("Light-red accent for the pause menu and the held-item box.")
        }
        (ModuleId::CustomUi, "Background") => {
            Some("Near-black fill behind the pause card and hotbar slots.")
        }
        (ModuleId::CustomUi, "Corner Radius") => {
            Some("Corner rounding of the hotbar slots and pause card.")
        }
        (ModuleId::CustomUi, "Edit Layout") => {
            Some("Open the drag-and-drop layout editor while the menu is open.")
        }

        (ModuleId::SpawnerFinder, "Notify") => {
            Some("Send a notification the moment a spawner is first seen.")
        }
        (ModuleId::SpawnerFinder, "Notify Range") => {
            Some("How close a spawner has to be to be announced.")
        }

        (ModuleId::TopBar, "Bar Height") => {
            Some("Height of the bar: every capsule, icon and label scales with it.")
        }
        (ModuleId::TopBar, "Show Brand") => Some("Show the Infered mark at the start of the bar."),
        (ModuleId::TopBar, "Show Wordmark") => Some("Show the Infered name beside the mark."),
        (ModuleId::TopBar, "Show Date") => Some("Show today's date."),
        (ModuleId::TopBar, "Show Ping") => {
            Some("Show the connection's latency, coloured by how good it is.")
        }
        (ModuleId::TopBar, "Show Coordinates") => Some("Show your position."),
        (ModuleId::TopBar, "Show Speed") => {
            Some("Show your horizontal speed in blocks per second.")
        }
        (ModuleId::TopBar, "Show Time") => Some("Show the clock."),
        (ModuleId::TopBar, "Show FPS") => Some("Show the frame rate."),
        (ModuleId::TopBar, "Show Direction") => Some("Show the way you are facing."),
        (ModuleId::TopBar, "Show Module Count") => Some("Show how many modules are switched on."),

        _ => None,
    }
}

 
fn common_help(name: &str) -> Option<&'static str> {
    Some(match name {
        "Accent Color" => "Accent color used by the card.",
        "Activation" => "How Freelook is activated: hold or toggle.",
        "Actual Mouse" => "Use real mouse movement for the aim instead of a synthetic path.",
        "Adaptive Strafe" => "Adjust strafing while the aura is active.",
        "Aim Settle (ms)" => "Time the aim spends settling before a crystal is placed.",
        "Aim Speed" => "How quickly the scaffold aim turns toward the target.",
        "Aim Time (ms)" => "Time spent aiming at a crystal before clicking.",
        "Aim Tolerance (blocks)" => "How close the aim must be before the action fires.",
        "Armor" => "Show the target's armor pieces.",
        "Art Corner Radius" => "Corner rounding of the album art.",
        "Art Size" => "Size of the album art.",
        "Artist Size" => "Font size of the artist line.",
        "Attack Delay (ms)" => "Wait between attacks.",
        "AuraGlobalSmoke" => "Enable the global smoke aura effect.",
        "Auto Block" => "Automatically place a block while scaffolding.",
        "Auto Jump" => "Jump automatically while moving.",
        "Auto Place Obsidian" => "Place obsidian under the crystal only when the clicked block is not already obsidian or bedrock.",
        "Auto Walk" => "Walk automatically toward the target.",
        "Background Opacity" => "Opacity of the card background.",
        "Bar Height" => "Height of the progress bar.",
        "Bhop" => "Add a bunny-hop boost while sprinting.",
        "Blocks" => "Blocks this module targets.",
        "Break Retry (ms)" => "Wait before retrying a failed crystal break.",
        "Break Timeout (ms)" => "Give up on a crystal break after this long.",
        "Broadcast" => "Announce this client to the Infered relay.",
        "Cancel Vanilla" => "Cancel the vanilla middle-click action.",
        "Cape" => "Cape artwork applied locally.",
        "Chance" => "Chance that a hit is extended.",
        "Click Button" => "Mouse button used by TriggerBot.",
        "Click Hold (ms)" => "How long the attack button is held.",
        "Click Randomization (ms)" => "Random jitter added to click timing.",
        "Color" => "Color used by this module's drawing.",
        "Color Tint" => "Tint applied to the smoke color.",
        "Combat Debug" => "Draw extra combat debug information.",
        "Connected" => "Show connected storage blocks too.",
        "Cooldown" => "Minimum time between actions.",
        "Cooldown Threshold" => "Only click when the attack cooldown is above this fraction.",
        "Core Burst" => "Show the bright core burst of the explosion.",
        "Core Color" => "Color of the explosion core.",
        "Corner Radius" => "Corner rounding of the drawn card.",
        "CPS" => "Clicks per second.",
        "Criticals Only" => "Only trigger on critical hits.",
        "Crystals" => "Allow triggering on end crystals.",
        "Ctrl Action" => "Action performed with Ctrl + middle click.",
        "Custom Command" => "Command run by the middle-click action.",
        "Debug Mode" => "Draw extra aiming and state diagnostics.",
        "Delay" => "Time to wait before acting.",
        "Delay (ms)" => "Time to wait before acting, in milliseconds.",
        "Choke" => "How long movement packets are held before they leave in a burst.",
        "Hold On KB" => "Use a longer packet hold after the server knocks you back.",
        "KB Hold Time" => "The temporary packet hold duration after knockback.",
        "Render" => "Draw the expanding ground ring that tracks the packet choke.",
        "Delay (ticks)" => "Time to wait before acting, in ticks.",
        "Direction" => "Direction the blur is applied.",
        "Distance" => "How far this feature reaches.",
        "Drop Junk" => "Drop junk items while managing the inventory.",
        "Durability" => "Show item durability.",
        "Duration" => "How long the effect lasts.",
        "Safe Walk" => "Sneak at a ledge and release 0.1 blocks before the edge.",
        "Ember Color" => "Color of the wing ember highlights.",
        "Expand Length" => "How far scaffold placement expands.",
        "Fade Time" => "How long the effect takes to fade.",
        "FOV" => "Field of view used by the aim.",
        "Gear" => "Show held items and armor.",
        "Glow" => "Strength of the glow.",
        "Glow Strength" => "Strength of the glow effect.",
        "GlowStrength" => "Strength of the glow effect.",
        "Hands" => "Show held items.",
        "Health" => "Show health information.",
        "Heartbeat" => "How often presence is refreshed.",
        "Height" => "Height of the drawn element.",
        "Hide Vanilla" => "Hide the vanilla explosion effect.",
        "Highlight Color" => "Color used for highlighted lyrics.",
        "Hit Chance (%)" => "Chance that an attack is clicked.",
        "Horizontal" => "Horizontal knockback reduction.",
        "HUD Scale" => "Size of the name-tag cards.",
        "HUD X Offset" => "Horizontal offset of name-tag cards.",
        "HUD Y Offset" => "Vertical offset of name-tag cards.",
        "Hunger Threshold" => "Eat when hunger falls to or below this level.",
        "HurtTime Check" => "Only click when the target can take a hit.",
        "Ignore Invisible" => "Skip invisible targets.",
        "Interval" => "Time between actions.",
        "Inventory Delay" => "Delay between inventory clicks.",
        "Invert Mouse" => "Invert the freelook camera's vertical mouse movement.",
        "Jump" => "Allow jumping while a screen is open.",
        "Lines Above" => "Lines of lyrics shown above the current line.",
        "Lines Below" => "Lines of lyrics shown below the current line.",
        "Lookahead (ticks)" => "How far ahead safewalk checks for edges.",
        "Max Delay" => "Maximum delay between actions.",
        "Max Delay (ms)" => "Maximum delay between actions.",
        "Max FPS" => "Frame-rate cap.",
        "Max Hold (ms)" => "Maximum time a click is held.",
        "Max HurtTime" => "Maximum target hurt time to click.",
        "Max Width" => "Maximum width of the card.",
        "Message" => "Chat message sent by the module.",
        "Mobs" => "Apply this to mobs.",
        "Min Delay" => "Minimum delay between actions.",
        "Min Distance" => "How far off a side face you must be for scaffold to click it.",
        "Min Delay (ms)" => "Minimum delay between actions.",
        "Mode" => "Operating mode.",
        "Move Back" => "Move back to the starting position.",
        "Multi Target" => "Allow acting on more than one target.",
        "Multiplier" => "Speed multiplier.",
        "Name" => "Label shown for this element.",
        "New Target Window (ms)" => "Time window for switching to a new target.",
        "Only Air" => "Only strafe while airborne.",
        "Only On Ground" => "Only act while on the ground.",
        "Only While Moving" => "Only act while moving.",
        "Only While Sprinting" => "Only act while sprinting.",
        "OrbColor" => "Color of the drifting orbs.",
        "OrbFlicker" => "How much the orbs flicker.",
        "Orbs" => "Show drifting smoke orbs.",
        "Padding" => "Inner padding of the card.",
        "Intensity" => "Amount of detail in the rendered effect.",
        "Pause While Using Item" => "Stop acting while an item is in use.",
        "Play Sound" => "Play the explosion sound.",
        "Players" => "Apply this to players.",
        "Players Only" => "Only show the effect for players.",
        "Pop Retry (ms)" => "Wait before retrying a crystal pop.",
        "Prediction" => "Predict target movement while aiming.",
        "Prefer Tier" => "Prefer the highest tool tier.",
        "Priority" => "Target priority used by the module.",
        "Progress Height" => "Height of the progress bar.",
        "Pulse" => "Make the effect pulse over time.",
        "Pulse Interval (ticks)" => "Ticks between sneak pulses.",
        "Pulse Jitter (ticks)" => "Random variation added to the pulse interval.",
        "Pulse Length (ticks)" => "Ticks each sneak pulse lasts.",
        "Radius" => "Radius of the effect.",
        "Rainbow" => "Cycle the effect through rainbow colors.",
        "Random Delay" => "Add random variation to the delay.",
        "Range" => "Maximum distance this feature acts within.",
        "Reaction Delay (ms)" => "Delay before reacting.",
        "Refill Hotbar" => "Refill the hotbar from the inventory.",
        "Release Delay (ticks)" => "Ticks to wait before releasing sneak.",
        "Remove Entity" => "Remove the entity after the effect.",
        "Require Mouse Hold" => "Only act while the mouse button is held.",
        "Require Reach" => "Only act when the target is within reach.",
        "Resume Delay (ticks)" => "Ticks to wait before resuming.",
        "Retry Attempts" => "Number of retries.",
        "Restore Crystal on Cancel" => "Switch back to the crystal when a sequence is abandoned before it places.",
        "Return Speed" => "How quickly the camera returns.",
        "Return Time (ms)" => "Time taken to return the camera.",
        "Return to View" => "Return the camera to the player's view.",
        "Saturation" => "Saturation multiplier.",
        "Scale" => "Scale of the drawn element.",
        "Self" => "Apply this to your own player model.",
        "Server Confirm Timeout" => "How long to wait for the server's own block update before giving up.",
        "Shade" => "Shade the rendered block.",
        "Shade Opacity" => "Opacity of the block shading.",
        "Shield Check" => "Check for a shield before attacking.",
        "Shift Action" => "Action performed with Shift + middle click.",
        "Shockwave & Sparks" => "Show the explosion shockwave and sparks.",
        "Show Album Art" => "Show album art in the lyrics card.",
        "Show Artist" => "Show the artist line.",
        "Show Background" => "Draw the card background.",
        "Show Border" => "Draw the card border.",
        "Show Brand" => "Show the Infered brand on the card.",
        "Show Direction" => "Show the direction indicator.",
        "Show Elapsed" => "Show elapsed playback time.",
        "Show FPS" => "Show the FPS counter.",
        "Show Impact" => "Show the predicted impact point.",
        "Show Logo" => "Show the Infered logo on other users' nametags.",
        "Show Mobs" => "Show mobs in the element.",
        "Show Module Count" => "Show the number of enabled modules.",
        "Show Other Entities" => "Show other entities in the stats.",
        "Show Progress Bar" => "Show the playback progress bar.",
        "Show Time" => "Show the current time.",
        "Show Title" => "Show the track title.",
        "Show When Empty" => "Keep the element visible when there is nothing to show.",
        "Silent Aim" => "Aim silently, without moving the visible camera.",
        "Size" => "Size of the effect.",
        "Slide Speed" => "Speed of the card's slide animation.",
        "Smear" => "Amount of motion smear.",
        "Smoke Cloud" => "Show the smoke cloud.",
        "Smoke Color" => "Color of the smoke.",
        "Smooth" => "Smooth out the movement.",
        "Smooth Return" => "Ease the camera back to view.",
        "Spacing" => "Spacing between elements.",
        "Spark Color" => "Color of the sparks.",
        "Spawn Key" => "Key that spawns the effect.",
        "Spawn on Key" => "Spawn the effect when the key is pressed.",
        "Spawn on Move" => "Spawn the effect while moving.",
        "Speed" => "Speed of the effect.",
        "Sprint Reset" => "Reset sprint before attacking.",
        "Stop In Water" => "Stop the effect in water.",
        "Stop On Screen" => "Stop the effect while a screen is open.",
        "Storage Blocks" => "Storage blocks to highlight.",
        "Strength" => "Strength of the effect.",
        "Style" => "Visual style of the effect.",
         
        "Band Color" => "Metal colour of the crown's band.",
        "Flame Color" => "Colour of the flame tongues and embers.",
        "Gem Color" => "Colour of the gems set into the band.",
        "Hover" => "How far above the head the crown floats.",
        "Hotbar Timeout" => "How long a hotbar switch may take before the sequence is abandoned.",
        "Points" => "Number of points around the crown.",
        "Spike Height" => "Height of the crown's points.",
        "Band Height" => "Height of the band around the head.",
        "Flame Length" => "How far the flames reach above the points.",
        "Flame Waviness" => "How much the flames sway sideways.",
        "Flame Speed" => "How fast the flames dance.",
        "Flicker" => "How much the flames and glow flicker.",
        "Spin Speed" => "How fast the crown turns around the head.",
        "Bob" => "How much the crown bobs up and down.",
        "Gems" => "Draw glowing gems around the band.",
        "Embers" => "Number of embers rising from the crown.",
        "Ember Speed" => "How fast the embers rise.",
        "Through Walls" | "ThroughWalls" => "Draw the effect through walls.",
        "Only Visible" | "OnlyVisible" => "Only draw the effect when the target is visible.",
        "Swap From Inventory" => "Swap the item from the inventory.",
        "Target Timeout (ms)" => "Drop a target after this long without a hit.",
        "Technique" => "Placement technique used by scaffold.",
        "Text Color" => "Color of the card text.",
        "Thickness" => "Thickness of the drawn line.",
        "Lines" => "Number of aura lines.",
        "Switches" => "One-block steps each line takes before it fades.",
        "Spread" => "How far a line may wander from you.",
        "Title Size" => "Font size of the track title.",
        "Tracers" => "Draw tracer lines for selected blocks.",
        "Vertical" => "Vertical knockback reduction.",
        "Visual Silent" => "Test setting: draw the silent aim on your own model in third person.",
        "Wait for Server Confirmation" => "Keep obsidian selected until the server confirms the base block before placing the crystal.",
        "Walk Range" => "Distance the aura walks within.",
        "Weapon Filter" => "Only trigger with the selected weapon.",
        "Width" => "Width of the drawn element.",
        "Wing Color" => "Membrane color of the wings.",
        "Word Highlight" => "Highlight the current word in the lyrics.",
        "Sky Lyrics" => "Place synced lyrics across the active Atmosphere sky.",
        "Zero Delay" => "Remove the delay between actions.",
         
        "Density" => "How much smoke each emission adds.",
        "Lifetime" => "How long each smoke puff lives.",
        "Wind" => "How strongly the smoke drifts sideways.",
        "Wind Direction" => "Direction the smoke drifts toward, in degrees.",
        "Opacity" => "Opacity of the drawn effect.",
        "AuraSmokeRadius" => "How far the aura's smoke spreads from each target.",
        "AuraSmokeOpacity" => "Opacity of the smoke around combat targets.",
        "AuraSmokeDensity" => "How much smoke surrounds combat targets.",
        "AuraSmokeFadeSpeed" => "How quickly aura smoke fades in and out.",
        "OrbCount" => "Number of drifting orbs.",
        "OrbSize" => "Size of each orb.",
        "OrbGlowStrength" => "Glow strength of the orbs.",
        "Sound Volume" => "Volume of the explosion sound.",
        "Red" => "Red channel of the sky color.",
        "Green" => "Green channel of the sky color.",
        "Blue" => "Blue channel of the sky color.",

         
        "Copies" => "Number of afterimages left behind each swing.",
        "Hold (s)" => "How long each afterimage stays at full opacity.",
        "Fade (s)" => "How long an afterimage takes to fade.",
        "Start Opacity" => "Opacity of the first afterimage.",
        "Smoothness" => "How smoothly the afterimages follow the swing.",
        "Pulse Speed" => "How fast the aura pulses.",
        "Samples" => "Number of samples used to draw the outline.",
        "Main Hand" => "Apply this to the main hand.",
        "Off Hand" => "Apply this to the off hand.",
        "Main X" | "Main Y" | "Main Z" => "Position offset of the main-hand item.",
        "Off X" | "Off Y" | "Off Z" => "Position offset of the off-hand item.",
        "Main Pitch" | "Main Yaw" | "Main Roll" => "Rotation of the main-hand item.",
        "Off Pitch" | "Off Yaw" | "Off Roll" => "Rotation of the off-hand item.",
        _ => return None,
    })
}

 
fn fallback_help(name: &str, setting: &ModuleSetting) -> String {
    match setting {
        ModuleSetting::Toggle { .. } => {
            if let Some(rest) = name.strip_prefix("Show ") {
                format!("Toggles whether {} is shown.", rest.to_ascii_lowercase())
            } else if let Some(rest) = name.strip_prefix("Only ") {
                format!("Restricts this feature to {}.", rest.to_ascii_lowercase())
            } else if let Some(rest) = name.strip_prefix("Ignore ") {
                format!("Ignores {}.", rest.to_ascii_lowercase())
            } else if let Some(rest) = name.strip_prefix("Require ") {
                format!("Only acts when {}.", rest.to_ascii_lowercase())
            } else if let Some(rest) = name.strip_prefix("Stop ") {
                format!("Stops the effect when {}.", rest.to_ascii_lowercase())
            } else {
                format!("Turns {} on or off.", name)
            }
        }
        ModuleSetting::Slider { min, max, .. } => {
            format!("Adjusts {}. Range: {min:.2} to {max:.2}.", name)
        }
        ModuleSetting::Choice { options, .. } => {
            format!("Selects {}. Options: {}.", name, options.join(", "))
        }
        ModuleSetting::Color { .. } => format!("Color used for {}.", name),
        ModuleSetting::BlockList { .. } => format!("Blocks selected for {}.", name),
        ModuleSetting::Text { .. } => format!("Text used for {}.", name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, Order, Pos2, RawInput, Rect, Vec2};

    #[test]
    fn a_note_appears_only_after_a_full_second() {
        assert!(!tooltip_due(10.0, 10.99));
        assert!(tooltip_due(10.0, 11.0));
        assert!(tooltip_due(10.0, 11.5));
    }

     
     
    #[test]
    fn the_tooltip_layer_waits_one_second() {
        let ctx = Context::default();
        let tip = Id::new("delayed_tooltip_test");
        let layer = LayerId::background();
        let frame = |time: f64| {
            let mut input = RawInput {
                time: Some(time),
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(640.0, 480.0))),
                ..Default::default()
            };
            input
                .events
                .push(Event::PointerMoved(Pos2::new(120.0, 120.0)));
            let mut shown = false;
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |_ui| {
                    delayed_tooltip(ctx, tip, layer, true, "Title", "Body");
                });
                shown = ctx.memory(|memory| {
                    memory
                        .areas()
                        .visible_layer_ids()
                        .iter()
                        .any(|layer| layer.order == Order::Tooltip)
                });
            });
            shown
        };

        assert!(!frame(0.0));
        assert!(!frame(0.5));
        assert!(frame(1.1));
    }

    #[test]
    fn module_specific_help_beats_the_shared_table() {
        let toggle = ModuleSetting::Toggle {
            name: "Health".into(),
            value: true,
        };
        assert!(setting_help(ModuleId::Nametags, "Health", &toggle).contains("name tag"));
        assert!(setting_help(ModuleId::TotemPanic, "Health", &toggle).contains("trigger"));
    }

    #[test]
    fn custom_ui_settings_explain_their_role() {
        let scale = ModuleSetting::Slider {
            name: "Hotbar Scale".into(),
            value: 1.0,
            min: 0.7,
            max: 1.6,
        };
        assert!(setting_help(ModuleId::CustomUi, "Hotbar Scale", &scale).contains("hotbar"));

        let slide = ModuleSetting::Slider {
            name: "Slide Speed".into(),
            value: 1.15,
            min: 0.4,
            max: 2.5,
        };
        assert!(setting_help(ModuleId::CustomUi, "Slide Speed", &slide).contains("slot"));

        let toggle = ModuleSetting::Toggle {
            name: "Edit Layout".into(),
            value: false,
        };
        assert!(setting_help(ModuleId::CustomUi, "Edit Layout", &toggle).contains("drag"));
    }

    #[test]
    fn unknown_settings_still_get_a_note_from_their_kind() {
        let slider = ModuleSetting::Slider {
            name: "Mystery Amount".into(),
            value: 0.5,
            min: 0.0,
            max: 1.0,
        };
        let text = setting_help(ModuleId::Fly, "Mystery Amount", &slider);
        assert!(text.contains("Adjusts Mystery Amount"));
        assert!(text.contains("0.00 to 1.00"));
    }

    #[test]
    fn chams_self_and_depth_settings_have_real_help() {
        let self_toggle = ModuleSetting::Toggle {
            name: "Self".into(),
            value: false,
        };
        let walls = ModuleSetting::Toggle {
            name: "ThroughWalls".into(),
            value: true,
        };
        let armor = ModuleSetting::Toggle {
            name: "Armor".into(),
            value: false,
        };
        assert!(setting_help(ModuleId::Chams, "Self", &self_toggle).contains("your own"));
        assert!(setting_help(ModuleId::Chams, "ThroughWalls", &walls).contains("walls"));
        assert!(setting_help(ModuleId::Chams, "Armor", &armor).contains("armor"));
    }

    #[test]
    fn crown_settings_explain_the_crown() {
        let slider = |name: &str| ModuleSetting::Slider {
            name: name.into(),
            value: 1.0,
            min: 0.0,
            max: 2.0,
        };
        for name in [
            "Style",
            "Core Color",
            "Glow",
            "Glow Strength",
            "Size",
            "Range",
            "Points",
        ] {
            let help = setting_help(ModuleId::Crown, name, &slider(name));
            assert!(!help.is_empty(), "{name} has no help");
            assert!(
                !help.starts_with("Adjusts"),
                "{name} fell back to its kind: {help}"
            );
        }
        assert!(setting_help(ModuleId::Crown, "Range", &slider("Range")).contains("crown"));
        assert!(setting_help(ModuleId::Crown, "Style", &slider("Style")).contains("Halo"));
    }

    #[test]
    fn common_settings_have_real_help() {
        let toggle = ModuleSetting::Toggle {
            name: "Show Logo".into(),
            value: true,
        };
        assert!(setting_help(ModuleId::InferedTag, "Show Logo", &toggle).contains("Infered logo"));
    }
}
