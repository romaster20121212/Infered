use crate::mapping::block::Direction;
use crate::mapping::client::chunk_batch::ChunkBatchSizeCalculator;
use crate::mapping::client::game_renderer::GameRenderer;
use crate::mapping::client::gamemode::MultiPlayerGameMode;
use crate::mapping::client::options::Options;
use crate::mapping::client::screen::Screen;
use crate::mapping::client::window::Window;
use crate::mapping::client::world::World;
use crate::mapping::entity::player::LocalPlayer;
use crate::mapping::math::BlockPos;
use crate::mapping::{FieldType, MappedObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::{GlobalRef, JObject, JValue};
use std::sync::RwLock;

 
 
 
 
 
 
 
#[derive(Debug, MappedObject)]
#[mapped(class = Minecraft)]
pub struct Minecraft {
    jni_ref: GlobalRef,
    pub window: Window,
     
    player: RwLock<Option<LocalPlayer>>,
     
     
    hidden_splash: RwLock<Option<GlobalRef>>,
}

impl Minecraft {
     
     
    pub fn new() -> anyhow::Result<Minecraft> {
        let jni_ref = mapping().in_frame(|| {
            let minecraft = mapping()
                .call_static_method(MinecraftClassType::Minecraft, "getInstance", &[])?
                .l()?;
            if minecraft.is_null() {
                return Err(anyhow::anyhow!("Minecraft.getInstance() returned null"));
            }
            mapping().new_global_ref(minecraft)
        })?;
        let window = Window::new(&jni_ref)?;

        Ok(Minecraft {
            jni_ref,
            window,
            player: RwLock::new(None),
            hidden_splash: RwLock::new(None),
        })
    }

     
     
     
     
    pub fn player(&self) -> anyhow::Result<Option<LocalPlayer>> {
        let Some(player_ref) = self.world_object("player", MinecraftClassType::LocalPlayer)? else {
             
             
            if let Ok(mut cache) = self.player.write() {
                *cache = None;
            }
            return Ok(None);
        };

        {
            let cache = self
                .player
                .read()
                .map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
            if let Some(cached) = cache.as_ref() {
                if mapping()
                    .get_env()?
                    .is_same_object(cached.jni_ref(), &player_ref)?
                {
                    return Ok(Some(cached.clone()));
                }
            }
        }

        let player = LocalPlayer::new(player_ref)?;
        *self
            .player
            .write()
            .map_err(|_| anyhow::anyhow!("Lock poisoned"))? = Some(player.clone());
        Ok(Some(player))
    }

     
    pub fn world(&self) -> anyhow::Result<Option<World>> {
        Ok(self
            .world_object("level", MinecraftClassType::Level)?
            .map(World::new))
    }

     
    pub fn game_mode(&self) -> anyhow::Result<Option<MultiPlayerGameMode>> {
        Ok(self
            .world_object("gameMode", MinecraftClassType::MultiPlayerGameMode)?
            .map(MultiPlayerGameMode::new))
    }

     
    pub fn crosshair_entity(&self) -> anyhow::Result<Option<crate::mapping::entity::Entity>> {
        self.in_frame(|| {
            let entity = self
                .get_field(
                    "crosshairPickEntity",
                    FieldType::Object(MinecraftClassType::Entity),
                )?
                .l()?;
            if entity.is_null() {
                return Ok(None);
            }
            Ok(Some(crate::mapping::entity::Entity::new(
                mapping().new_global_ref(entity)?,
            )))
        })
    }

     
    pub fn crosshair_entity_id(&self) -> anyhow::Result<Option<i32>> {
        self.in_frame(|| {
            let entity = self
                .get_field(
                    "crosshairPickEntity",
                    FieldType::Object(MinecraftClassType::Entity),
                )?
                .l()?;
            if entity.is_null() {
                return Ok(None);
            }
            Ok(Some(
                mapping()
                    .call_method(MinecraftClassType::Entity, &entity, "getId", &[])?
                    .i()?,
            ))
        })
    }

     
     
    pub fn hit_block(&self) -> anyhow::Result<Option<BlockPos>> {
        self.in_frame(|| {
            let hit = mapping()
                .get_field(
                    MinecraftClassType::Minecraft,
                    self.jni_ref().as_obj(),
                    "hitResult",
                    FieldType::Object(MinecraftClassType::HitResult),
                )?
                .l()?;
            if hit.is_null()
                || !mapping().is_instance_of(MinecraftClassType::BlockHitResult, &hit)?
            {
                return Ok(None);
            }
            let kind = mapping()
                .call_method(MinecraftClassType::HitResult, &hit, "getType", &[])?
                .l()?;
            let mut env = mapping().get_env()?;
            let kind_name = env
                .call_method(&kind, "name", "()Ljava/lang/String;", &[])?
                .l()?;
            let kind_name = jni::objects::JString::from(kind_name);
            if env.get_string(&kind_name)?.to_string_lossy() != "BLOCK" {
                return Ok(None);
            }
            let pos = mapping()
                .call_method(MinecraftClassType::BlockHitResult, &hit, "getBlockPos", &[])?
                .l()?;
            if pos.is_null() {
                return Ok(None);
            }
            Ok(Some(BlockPos::read(&pos)?))
        })
    }

     
     
    pub fn hit_block_in_reach(&self) -> anyhow::Result<Option<BlockPos>> {
        Ok(self.hit_block_face_in_reach()?.map(|(pos, _, _)| pos))
    }

     
     
    pub fn hit_block_face_in_reach(
        &self,
    ) -> anyhow::Result<Option<(BlockPos, Direction, crate::mapping::math::Vec3)>> {
        self.in_frame(|| {
            let Some(pos) = self.hit_block()? else {
                return Ok(None);
            };
            let Some(player) = self.player()? else {
                return Ok(None);
            };
            let hit = self
                .get_field(
                    "hitResult",
                    FieldType::Object(MinecraftClassType::HitResult),
                )?
                .l()?;
            let location = mapping()
                .call_method(MinecraftClassType::HitResult, &hit, "getLocation", &[])?
                .l()?;
            let location = crate::mapping::math::Vec3::read(&location)?;
            let direction = mapping()
                .call_method(
                    MinecraftClassType::BlockHitResult,
                    &hit,
                    "getDirection",
                    &[],
                )?
                .l()?;
            let mut env = mapping().get_env()?;
            let direction_name = env
                .call_method(&direction, "name", "()Ljava/lang/String;", &[])?
                .l()?;
            let direction_name = jni::objects::JString::from(direction_name);
            let direction_name = env
                .get_string(&direction_name)?
                .to_string_lossy()
                .into_owned();
            let Some(face) = Direction::from_jvm_name(&direction_name) else {
                return Ok(None);
            };
            use crate::mapping::entity::EntityRef;
            let eye = player.get_eye_position()?;
            let range = player.call_method("blockInteractionRange", &[])?.d()?;
            let distance2 = (location.x() - eye.x()).powi(2)
                + (location.y() - eye.y()).powi(2)
                + (location.z() - eye.z()).powi(2);
            if !range.is_finite()
                || range < 0.0
                || !distance2.is_finite()
                || distance2 > (range + 0.001).powi(2)
            {
                return Ok(None);
            }
            Ok(Some((pos, face, location)))
        })
    }

     
    pub fn game_renderer(&self) -> anyhow::Result<GameRenderer> {
        self.in_frame(|| {
            let obj = self
                .get_field(
                    "gameRenderer",
                    FieldType::Object(MinecraftClassType::GameRenderer),
                )?
                .l()?;
            Ok(GameRenderer::new(mapping().new_global_ref(obj)?))
        })
    }

     
    pub fn options(&self) -> anyhow::Result<Options> {
        self.in_frame(|| {
            let obj = self
                .get_field("options", FieldType::Object(MinecraftClassType::Options))?
                .l()?;
            Ok(Options::new(mapping().new_global_ref(obj)?))
        })
    }

     
     
     
     
     
    pub fn chunk_batch_calculator(&self) -> anyhow::Result<Option<ChunkBatchSizeCalculator>> {
        self.in_frame(|| {
            let listener = self.call_method("getConnection", &[])?.l()?;
            if listener.is_null() {
                return Ok(None);
            }
            let calculator = mapping()
                .call_method(
                    MinecraftClassType::ClientPacketListener,
                    &listener,
                    "getChunkBatchSizeCalculator",
                    &[],
                )
                .or_else(|_| {
                     
                     
                    mapping().get_field(
                        MinecraftClassType::ClientPacketListener,
                        &listener,
                        "chunkBatchSizeCalculator",
                        FieldType::Object(MinecraftClassType::ChunkBatchSizeCalculator),
                    )
                })?
                .l()?;
            if calculator.is_null() {
                return Ok(None);
            }
            Ok(Some(ChunkBatchSizeCalculator::new(
                mapping().new_global_ref(calculator)?,
            )))
        })
    }

     
     
     
     
    pub fn start_attack(&self) -> anyhow::Result<()> {
        self.call_method("startAttack", &[])?;
        Ok(())
    }

     
    pub fn start_use_item(&self) -> anyhow::Result<()> {
        self.call_method("startUseItem", &[])?;
        Ok(())
    }

     
    pub fn pick_block(&self) -> anyhow::Result<()> {
        self.call_method("pickBlock", &[])?;
        Ok(())
    }

     
    pub fn in_world(&self) -> bool {
        matches!(self.player(), Ok(Some(_)))
    }

     
     
     
     
    pub fn send_chat(&self, message: &str) -> anyhow::Result<()> {
        self.send_on_connection("sendChat", message)
    }

     
     
     
     
     
    pub fn send_command(&self, command: &str) -> anyhow::Result<()> {
        self.send_on_connection("sendCommand", command)
    }

     
     
    fn send_on_connection(&self, method: &str, text: &str) -> anyhow::Result<()> {
        self.in_frame(|| {
            let listener = self.call_method("getConnection", &[])?.l()?;
            if listener.is_null() {
                anyhow::bail!("not connected to a server");
            }
            let text = mapping().get_env()?.new_string(text)?;
            mapping().call_method(
                MinecraftClassType::ClientPacketListener,
                &listener,
                method,
                &[JValue::Object(&text)],
            )?;
            Ok(())
        })
    }

     
     
    pub fn game_profile(&self) -> anyhow::Result<jni::objects::GlobalRef> {
        self.in_frame(|| {
            let profile = self.call_method("getGameProfile", &[])?.l()?;
            if profile.is_null() {
                return Err(anyhow::anyhow!("Minecraft.getGameProfile() returned null"));
            }
            mapping().new_global_ref(profile)
        })
    }

     
     
    pub fn teardown(&self) {
        if let Ok(mut cache) = self.player.write() {
            *cache = None;
        }
    }

     
    pub fn open_inventory_screen(&self, player: &LocalPlayer) -> anyhow::Result<()> {
        self.in_frame(|| {
            let mut env = mapping().get_env()?;
            let class = mapping().resolve_class(
                &mut env,
                &mapping().runtime_class_name(MinecraftClassType::InventoryScreen)?,
            )?;
            let signature = format!(
                "(L{};)V",
                mapping().runtime_class_name(MinecraftClassType::Player)?
            );
            let screen = env.new_object(
                &class,
                &signature,
                &[JValue::Object(player.jni_ref().as_obj())],
            )?;
            self.call_method("setScreen", &[JValue::Object(&screen)])?;
            Ok(())
        })
    }

     
    pub fn close_screen(&self) -> anyhow::Result<()> {
        self.in_frame(|| {
            self.call_method("setScreen", &[JValue::Object(&JObject::null())])?;
            Ok(())
        })
    }

     
    pub fn current_screen_is_null(&self) -> bool {
        self.current_screen() == Screen::None
    }

     
     
     
     
     
     
     
     
     
     
     
     
    pub fn screen_is_null(&self) -> bool {
        self.in_frame(|| {
            Ok(self
                .get_field("screen", FieldType::Object(MinecraftClassType::Screen))?
                .l()?
                .is_null())
        })
        .unwrap_or(false)
    }

     
     
     
     
     
    pub fn title_screen_open(&self) -> bool {
        self.in_frame(|| {
            let screen = self
                .get_field("screen", FieldType::Object(MinecraftClassType::Screen))?
                .l()?;
            if screen.is_null() {
                return Ok(false);
            }
            Ok(mapping()
                .is_instance_of(MinecraftClassType::TitleScreen, &screen)
                .unwrap_or(false))
        })
        .unwrap_or(false)
    }

     
     
     
    fn open_title_screen(&self) -> anyhow::Result<Option<JObject<'static>>> {
        let screen = self
            .get_field("screen", FieldType::Object(MinecraftClassType::Screen))?
            .l()?;
        if screen.is_null()
            || !mapping()
                .is_instance_of(MinecraftClassType::TitleScreen, &screen)
                .unwrap_or(false)
        {
            return Ok(None);
        }
        Ok(Some(screen))
    }

     
     
     
     
     
    pub fn title_splash(&self) -> Option<String> {
        self.in_frame(|| {
            let Some(screen) = self.open_title_screen()? else {
                return Ok(None);
            };
            let splash = mapping()
                .get_field(
                    MinecraftClassType::TitleScreen,
                    &screen,
                    "splash",
                    FieldType::Object(MinecraftClassType::SplashRenderer),
                )?
                .l()?;
            if splash.is_null() {
                return Ok(None);
            }
            let text = mapping()
                .get_field(
                    MinecraftClassType::SplashRenderer,
                    &splash,
                    "splash",
                    FieldType::String,
                )?
                .l()?;
            if text.is_null() {
                return Ok(None);
            }
            Ok(Some(mapping().get_string(text)?))
        })
        .unwrap_or(None)
    }

     
     
     
     
     
     
    pub fn hide_title_splash(&self) -> anyhow::Result<()> {
        self.in_frame(|| {
            let Some(screen) = self.open_title_screen()? else {
                return Ok(());
            };
            let splash = mapping()
                .get_field(
                    MinecraftClassType::TitleScreen,
                    &screen,
                    "splash",
                    FieldType::Object(MinecraftClassType::SplashRenderer),
                )?
                .l()?;
            if splash.is_null() {
                 
                return Ok(());
            }
            let saved = mapping().new_global_ref(splash)?;
            *self
                .hidden_splash
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(saved);
            mapping().set_field(
                MinecraftClassType::TitleScreen,
                &screen,
                "splash",
                FieldType::Object(MinecraftClassType::SplashRenderer),
                JValue::Object(&JObject::null()),
            )
        })
    }

     
     
     
    pub fn restore_title_splash(&self) -> anyhow::Result<()> {
        let saved = self
            .hidden_splash
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        let Some(saved) = saved else {
            return Ok(());
        };
        self.in_frame(|| {
            let Some(screen) = self.open_title_screen()? else {
                return Ok(());
            };
            let splash = mapping()
                .get_field(
                    MinecraftClassType::TitleScreen,
                    &screen,
                    "splash",
                    FieldType::Object(MinecraftClassType::SplashRenderer),
                )?
                .l()?;
            if !splash.is_null() {
                 
                return Ok(());
            }
            mapping().set_field(
                MinecraftClassType::TitleScreen,
                &screen,
                "splash",
                FieldType::Object(MinecraftClassType::SplashRenderer),
                JValue::Object(saved.as_obj()),
            )
        })
    }

     
     
     
     
    pub fn current_screen_object(&self) -> anyhow::Result<Option<GlobalRef>> {
        self.in_frame(|| {
            let screen = self
                .get_field("screen", FieldType::Object(MinecraftClassType::Screen))?
                .l()?;
            if screen.is_null() {
                return Ok(None);
            }
            Ok(Some(mapping().new_global_ref(screen)?))
        })
    }

     
     
     
    pub fn current_screen(&self) -> Screen {
        self.in_frame(|| {
            let screen = self
                .get_field("screen", FieldType::Object(MinecraftClassType::Screen))?
                .l()?;
            if screen.is_null() {
                return Ok(Screen::None);
            }

            let is = |class| mapping().is_instance_of(class, &screen).unwrap_or(false);
             
             
            let screen = if is(MinecraftClassType::ChatScreen) {
                Screen::Chat
            } else if is(MinecraftClassType::CreativeModeInventoryScreen)
                || is(MinecraftClassType::InventoryScreen)
            {
                Screen::Inventory
            } else if is(MinecraftClassType::CraftingScreen) {
                Screen::Crafting
            } else if is(MinecraftClassType::AbstractContainerScreen) {
                Screen::Container
            } else if is(MinecraftClassType::PauseScreen) {
                Screen::Menu
            } else if is(MinecraftClassType::DisconnectedScreen) {
                Screen::Disconnected
            } else {
                Screen::Unknown
            };
            Ok(screen)
        })
        .unwrap_or(Screen::Unknown)
    }

     
     
    pub fn current_server(&self) -> anyhow::Result<Option<GlobalRef>> {
        self.in_frame(|| {
            let server = self.call_method("getCurrentServer", &[])?.l()?;
            if server.is_null() {
                return Ok(None);
            }
            Ok(Some(mapping().new_global_ref(server)?))
        })
    }

     
     
    fn world_object(
        &self,
        field: &str,
        class: MinecraftClassType,
    ) -> anyhow::Result<Option<GlobalRef>> {
        self.in_frame(|| {
            let obj = self.get_field(field, FieldType::Object(class))?.l()?;
            if obj.is_null() {
                return Ok(None);
            }
            Ok(Some(mapping().new_global_ref(obj)?))
        })
    }
}
