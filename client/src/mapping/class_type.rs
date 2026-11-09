use std::fmt;

#[allow(dead_code)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MinecraftClassType {
    Minecraft,
    LocalPlayer,
    Level,
    Player,
    Abilities,
    Entity,
    Vec3,
    Window,
    MultiPlayerGameMode,
    Iterable,
    Iterator,
    Mob,
    Screen,
    Gui,
    GameRenderer,
    LevelRenderer,
    SkyRenderer,
    Frustum,
    SectionOcclusionGraph,
    Camera,
    CameraType,
    MouseHandler,
    LivingEntity,
    Component,
    LevelReader,
    LevelChunk,
    LevelChunkSection,
    PalettedContainer,
    PalettedContainerData,
    Palette,
    BitStorage,
    SimpleBitStorage,
    Registry,
    DefaultedRegistry,
    BuiltInRegistries,
    ResourceLocation,
    Block,
    BlockStateBase,
    StateHolder,
    Property,
    StateDefinition,
    VoxelShape,
    Aabb,
    DirectionAxis,
    BlockEntity,
    ChestBlockEntity,
    EnderChestBlockEntity,
    BarrelBlockEntity,
    ShulkerBoxBlockEntity,
    MobSpawnerBlockEntity,
    HopperBlockEntity,
    AbstractFurnaceBlockEntity,
    DropperBlockEntity,
    DispenserBlockEntity,
    ItemEntity,
    Projectile,
    Fireball,
    FishingHook,
    EndCrystal,
    MobEffectInstance,
    Holder,
    ResourceKey,
    Optional,
    BlockPos,
    Vec3i,
    Map,
    Object,
    File,
    PlayerInfo,
    PlayerSkin,
     
     
    AbstractClientPlayer,
    ResourceManager,
    Resource,
    InputStream,
    GameType,
    Options,
    OptionInstance,
    KeyMapping,
    InputConstants,
     
    InputConstantsKey,
    Integer,
    Double,
    Boolean,
    InteractionHand,
    InteractionResult,
    Inventory,
    FoodData,
    DataComponents,
    DataComponentType,
    PotionContents,
    ItemStack,
    Item,
     
     
    BlockItem,
    Items,
    AbstractContainerMenu,
    ContainerInput,
    EquipmentSlot,
    BlockState,
    Direction,
    HitResult,
    BlockHitResult,
    ChatScreen,
    PauseScreen,
     
    TitleScreen,
    LogoRenderer,
     
    SplashRenderer,
     
    AbstractWidget,
    Button,
     
     
    DisconnectedScreen,
    ConnectScreen,
    ServerData,
    ServerAddress,
    AbstractContainerScreen,
    InventoryScreen,
    CraftingScreen,
    CreativeModeInventoryScreen,
    ClientPacketListener,
    Connection,
    Packet,
     
     
    ChunkBatchSizeCalculator,
    IntegratedServer,
    MinecraftServer,
    PlayerList,
     
    Attributes,
    AttributeInstance,
    AttributeModifier,
    AttributeModifierOperation,
     
    ServerboundMovePlayerPacket,
    ServerboundMovePlayerPacketPos,
    ServerboundMovePlayerPacketPosRot,
    ServerboundMovePlayerPacketRot,
    ServerboundMovePlayerPacketStatusOnly,
    ServerboundInteractPacket,
    ServerboundInteractPacketAction,
    ServerboundSwingPacket,
    ServerboundSetCarriedItemPacket,
    ServerboundUseItemPacket,
    ClientboundSetEntityMotionPacket,
    ClientboundBlockUpdatePacket,
    ClientboundPlayerInfoUpdatePacket,
    ClientboundPlayerPositionPacket,
    ClientboundPlayerRotationPacket,
     
     
     
    ServerboundCustomPayloadPacket,
    ClientboundCustomPayloadPacket,
    BrandPayload,
    CustomPacketPayload,
     
     
     
    ClientboundExplodePacket,
     
    ClientboundSystemChatPacket,
    ClientboundPlayerChatPacket,
    SignedMessageBody,
    SoundEvent,
    SoundEvents,
    SoundSource,
     
    DustParticleOptions,
    RenderShape,
    BlockRenderDispatcher,
    BlockEntityRenderDispatcher,
     
     
    ItemInHandRenderer,
     
     
    ScreenEffectRenderer,
     
     
    ParticleEngine,
}

impl MinecraftClassType {
    pub fn get_name(&self) -> &str {
        match self {
            MinecraftClassType::Minecraft => "net/minecraft/client/Minecraft",
            MinecraftClassType::LocalPlayer => "net/minecraft/client/player/LocalPlayer",
            MinecraftClassType::Level => "net/minecraft/client/multiplayer/ClientLevel",
            MinecraftClassType::Player => "net/minecraft/world/entity/player/Player",
            MinecraftClassType::AbstractClientPlayer => {
                "net/minecraft/client/player/AbstractClientPlayer"
            }
            MinecraftClassType::ResourceManager => {
                "net/minecraft/server/packs/resources/ResourceManager"
            }
            MinecraftClassType::Resource => "net/minecraft/server/packs/resources/Resource",
            MinecraftClassType::InputStream => "java/io/InputStream",
            MinecraftClassType::Abilities => "net/minecraft/world/entity/player/Abilities",
            MinecraftClassType::Entity => "net/minecraft/world/entity/Entity",
            MinecraftClassType::Vec3 => "net/minecraft/world/phys/Vec3",
            MinecraftClassType::Window => "com/mojang/blaze3d/platform/Window",
            MinecraftClassType::MultiPlayerGameMode => {
                "net/minecraft/client/multiplayer/MultiPlayerGameMode"
            }
            MinecraftClassType::Iterable => "java/lang/Iterable",
            MinecraftClassType::Iterator => "java/util/Iterator",
            MinecraftClassType::Mob => "net/minecraft/world/entity/Mob",
            MinecraftClassType::Screen => "net/minecraft/client/gui/screens/Screen",
            MinecraftClassType::Gui => "net/minecraft/client/gui/Gui",
            MinecraftClassType::GameRenderer => "net/minecraft/client/renderer/GameRenderer",
            MinecraftClassType::LevelRenderer => "net/minecraft/client/renderer/LevelRenderer",
            MinecraftClassType::SkyRenderer => "net/minecraft/client/renderer/SkyRenderer",
            MinecraftClassType::Frustum => "net/minecraft/client/renderer/culling/Frustum",
            MinecraftClassType::SectionOcclusionGraph => {
                "net/minecraft/client/renderer/SectionOcclusionGraph"
            }
            MinecraftClassType::Camera => "net/minecraft/client/Camera",
            MinecraftClassType::CameraType => "net/minecraft/client/CameraType",
            MinecraftClassType::MouseHandler => "net/minecraft/client/MouseHandler",
            MinecraftClassType::LivingEntity => "net/minecraft/world/entity/LivingEntity",
            MinecraftClassType::Component => "net/minecraft/network/chat/Component",
            MinecraftClassType::LevelReader => "net/minecraft/world/level/LevelReader",
            MinecraftClassType::LevelChunk => "net/minecraft/world/level/chunk/LevelChunk",
            MinecraftClassType::LevelChunkSection => {
                "net/minecraft/world/level/chunk/LevelChunkSection"
            }
            MinecraftClassType::PalettedContainer => {
                "net/minecraft/world/level/chunk/PalettedContainer"
            }
            MinecraftClassType::PalettedContainerData => {
                "net/minecraft/world/level/chunk/PalettedContainer$Data"
            }
            MinecraftClassType::Palette => "net/minecraft/world/level/chunk/Palette",
            MinecraftClassType::BitStorage => "net/minecraft/util/BitStorage",
            MinecraftClassType::SimpleBitStorage => "net/minecraft/util/SimpleBitStorage",
            MinecraftClassType::Registry => "net/minecraft/core/Registry",
            MinecraftClassType::DefaultedRegistry => "net/minecraft/core/DefaultedRegistry",
            MinecraftClassType::BuiltInRegistries => {
                "net/minecraft/core/registries/BuiltInRegistries"
            }
            MinecraftClassType::ResourceLocation => "net/minecraft/resources/ResourceLocation",
            MinecraftClassType::Block => "net/minecraft/world/level/block/Block",
            MinecraftClassType::StateDefinition => {
                "net/minecraft/world/level/block/state/StateDefinition"
            }
            MinecraftClassType::BlockStateBase => {
                "net/minecraft/world/level/block/state/BlockBehaviour$BlockStateBase"
            }
            MinecraftClassType::StateHolder => "net/minecraft/world/level/block/state/StateHolder",
            MinecraftClassType::Property => {
                "net/minecraft/world/level/block/state/properties/Property"
            }
            MinecraftClassType::VoxelShape => "net/minecraft/world/phys/shapes/VoxelShape",
            MinecraftClassType::Aabb => "net/minecraft/world/phys/AABB",
            MinecraftClassType::DirectionAxis => "net/minecraft/core/Direction$Axis",
            MinecraftClassType::BlockEntity => "net/minecraft/world/level/block/entity/BlockEntity",
            MinecraftClassType::ChestBlockEntity => {
                "net/minecraft/world/level/block/entity/ChestBlockEntity"
            }
            MinecraftClassType::EnderChestBlockEntity => {
                "net/minecraft/world/level/block/entity/EnderChestBlockEntity"
            }
            MinecraftClassType::BarrelBlockEntity => {
                "net/minecraft/world/level/block/entity/BarrelBlockEntity"
            }
            MinecraftClassType::ShulkerBoxBlockEntity => {
                "net/minecraft/world/level/block/entity/ShulkerBoxBlockEntity"
            }
            MinecraftClassType::MobSpawnerBlockEntity => {
                "net/minecraft/world/level/block/entity/SpawnerBlockEntity"
            }
            MinecraftClassType::HopperBlockEntity => {
                "net/minecraft/world/level/block/entity/HopperBlockEntity"
            }
            MinecraftClassType::AbstractFurnaceBlockEntity => {
                "net/minecraft/world/level/block/entity/AbstractFurnaceBlockEntity"
            }
            MinecraftClassType::DropperBlockEntity => {
                "net/minecraft/world/level/block/entity/DropperBlockEntity"
            }
            MinecraftClassType::DispenserBlockEntity => {
                "net/minecraft/world/level/block/entity/DispenserBlockEntity"
            }
            MinecraftClassType::ItemEntity => "net/minecraft/world/entity/item/ItemEntity",
            MinecraftClassType::Projectile => "net/minecraft/world/entity/projectile/Projectile",
            MinecraftClassType::Fireball => "net/minecraft/world/entity/projectile/Fireball",
            MinecraftClassType::FishingHook => "net/minecraft/world/entity/projectile/FishingHook",
            MinecraftClassType::EndCrystal => {
                "net/minecraft/world/entity/boss/enderdragon/EndCrystal"
            }
            MinecraftClassType::MobEffectInstance => "net/minecraft/world/effect/MobEffectInstance",
            MinecraftClassType::Holder => "net/minecraft/core/Holder",
            MinecraftClassType::ResourceKey => "net/minecraft/resources/ResourceKey",
            MinecraftClassType::Optional => "java/util/Optional",
            MinecraftClassType::BlockPos => "net/minecraft/core/BlockPos",
            MinecraftClassType::Vec3i => "net/minecraft/core/Vec3i",
            MinecraftClassType::Map => "java/util/Map",
            MinecraftClassType::Object => "java/lang/Object",
            MinecraftClassType::File => "java/io/File",
            MinecraftClassType::PlayerInfo => "net/minecraft/client/multiplayer/PlayerInfo",
            MinecraftClassType::PlayerSkin => "net/minecraft/client/resources/PlayerSkin",
            MinecraftClassType::GameType => "net/minecraft/world/level/GameType",
            MinecraftClassType::Options => "net/minecraft/client/Options",
            MinecraftClassType::OptionInstance => "net/minecraft/client/OptionInstance",
            MinecraftClassType::KeyMapping => "net/minecraft/client/KeyMapping",
            MinecraftClassType::InputConstants => "com/mojang/blaze3d/platform/InputConstants",
            MinecraftClassType::InputConstantsKey => {
                "com/mojang/blaze3d/platform/InputConstants$Key"
            }
            MinecraftClassType::Integer => "java/lang/Integer",
            MinecraftClassType::Double => "java/lang/Double",
            MinecraftClassType::Boolean => "java/lang/Boolean",
            MinecraftClassType::InteractionHand => "net/minecraft/world/InteractionHand",
            MinecraftClassType::InteractionResult => "net/minecraft/world/InteractionResult",
            MinecraftClassType::Inventory => "net/minecraft/world/entity/player/Inventory",
            MinecraftClassType::FoodData => "net/minecraft/world/food/FoodData",
            MinecraftClassType::DataComponents => "net/minecraft/core/component/DataComponents",
            MinecraftClassType::DataComponentType => {
                "net/minecraft/core/component/DataComponentType"
            }
            MinecraftClassType::PotionContents => "net/minecraft/world/item/alchemy/PotionContents",
            MinecraftClassType::ItemStack => "net/minecraft/world/item/ItemStack",
            MinecraftClassType::Item => "net/minecraft/world/item/Item",
            MinecraftClassType::BlockItem => "net/minecraft/world/item/BlockItem",
            MinecraftClassType::Items => "net/minecraft/world/item/Items",
            MinecraftClassType::AbstractContainerMenu => {
                "net/minecraft/world/inventory/AbstractContainerMenu"
            }
            MinecraftClassType::ContainerInput => "net/minecraft/world/inventory/ContainerInput",
            MinecraftClassType::EquipmentSlot => "net/minecraft/world/entity/EquipmentSlot",
            MinecraftClassType::BlockState => "net/minecraft/world/level/block/state/BlockState",
            MinecraftClassType::Direction => "net/minecraft/core/Direction",
            MinecraftClassType::HitResult => "net/minecraft/world/phys/HitResult",
            MinecraftClassType::BlockHitResult => "net/minecraft/world/phys/BlockHitResult",
            MinecraftClassType::ChatScreen => "net/minecraft/client/gui/screens/ChatScreen",
            MinecraftClassType::PauseScreen => "net/minecraft/client/gui/screens/PauseScreen",
            MinecraftClassType::TitleScreen => "net/minecraft/client/gui/screens/TitleScreen",
            MinecraftClassType::LogoRenderer => "net/minecraft/client/gui/components/LogoRenderer",
            MinecraftClassType::SplashRenderer => {
                "net/minecraft/client/gui/components/SplashRenderer"
            }
            MinecraftClassType::AbstractWidget => {
                "net/minecraft/client/gui/components/AbstractWidget"
            }
            MinecraftClassType::Button => "net/minecraft/client/gui/components/Button",
            MinecraftClassType::DisconnectedScreen => {
                "net/minecraft/client/gui/screens/DisconnectedScreen"
            }
            MinecraftClassType::ConnectScreen => "net/minecraft/client/gui/screens/ConnectScreen",
            MinecraftClassType::ServerData => "net/minecraft/client/multiplayer/ServerData",
            MinecraftClassType::ServerAddress => "net/minecraft/client/multiplayer/ServerAddress",
            MinecraftClassType::AbstractContainerScreen => {
                "net/minecraft/client/gui/screens/inventory/AbstractContainerScreen"
            }
            MinecraftClassType::InventoryScreen => {
                "net/minecraft/client/gui/screens/inventory/InventoryScreen"
            }
            MinecraftClassType::CraftingScreen => {
                "net/minecraft/client/gui/screens/inventory/CraftingScreen"
            }
            MinecraftClassType::CreativeModeInventoryScreen => {
                "net/minecraft/client/gui/screens/inventory/CreativeModeInventoryScreen"
            }
            MinecraftClassType::ClientPacketListener => {
                "net/minecraft/client/multiplayer/ClientPacketListener"
            }
            MinecraftClassType::Connection => "net/minecraft/network/Connection",
            MinecraftClassType::Packet => "net/minecraft/network/protocol/Packet",
            MinecraftClassType::ChunkBatchSizeCalculator => {
                "net/minecraft/client/multiplayer/ChunkBatchSizeCalculator"
            }
            MinecraftClassType::IntegratedServer => "net/minecraft/client/server/IntegratedServer",
            MinecraftClassType::MinecraftServer => "net/minecraft/server/MinecraftServer",
            MinecraftClassType::PlayerList => "net/minecraft/server/players/PlayerList",
            MinecraftClassType::Attributes => "net/minecraft/world/entity/ai/attributes/Attributes",
            MinecraftClassType::AttributeInstance => {
                "net/minecraft/world/entity/ai/attributes/AttributeInstance"
            }
            MinecraftClassType::AttributeModifier => {
                "net/minecraft/world/entity/ai/attributes/AttributeModifier"
            }
            MinecraftClassType::AttributeModifierOperation => {
                "net/minecraft/world/entity/ai/attributes/AttributeModifier$Operation"
            }
            MinecraftClassType::ServerboundMovePlayerPacket => {
                "net/minecraft/network/protocol/game/ServerboundMovePlayerPacket"
            }
            MinecraftClassType::ServerboundMovePlayerPacketPos => {
                "net/minecraft/network/protocol/game/ServerboundMovePlayerPacket$Pos"
            }
            MinecraftClassType::ServerboundMovePlayerPacketPosRot => {
                "net/minecraft/network/protocol/game/ServerboundMovePlayerPacket$PosRot"
            }
            MinecraftClassType::ServerboundMovePlayerPacketRot => {
                "net/minecraft/network/protocol/game/ServerboundMovePlayerPacket$Rot"
            }
            MinecraftClassType::ServerboundMovePlayerPacketStatusOnly => {
                "net/minecraft/network/protocol/game/ServerboundMovePlayerPacket$StatusOnly"
            }
            MinecraftClassType::ServerboundInteractPacket => {
                "net/minecraft/network/protocol/game/ServerboundInteractPacket"
            }
            MinecraftClassType::ServerboundInteractPacketAction => {
                "net/minecraft/network/protocol/game/ServerboundInteractPacket$Action"
            }
            MinecraftClassType::ServerboundSwingPacket => {
                "net/minecraft/network/protocol/game/ServerboundSwingPacket"
            }
            MinecraftClassType::ServerboundSetCarriedItemPacket => {
                "net/minecraft/network/protocol/game/ServerboundSetCarriedItemPacket"
            }
            MinecraftClassType::ServerboundUseItemPacket => {
                "net/minecraft/network/protocol/game/ServerboundUseItemPacket"
            }
            MinecraftClassType::ClientboundBlockUpdatePacket => {
                "net/minecraft/network/protocol/game/ClientboundBlockUpdatePacket"
            }
            MinecraftClassType::ClientboundSetEntityMotionPacket => {
                "net/minecraft/network/protocol/game/ClientboundSetEntityMotionPacket"
            }
            MinecraftClassType::ClientboundPlayerInfoUpdatePacket => {
                "net/minecraft/network/protocol/game/ClientboundPlayerInfoUpdatePacket"
            }
            MinecraftClassType::ClientboundPlayerPositionPacket => {
                "net/minecraft/network/protocol/game/ClientboundPlayerPositionPacket"
            }
            MinecraftClassType::ClientboundPlayerRotationPacket => {
                "net/minecraft/network/protocol/game/ClientboundPlayerRotationPacket"
            }
            MinecraftClassType::ServerboundCustomPayloadPacket => {
                "net/minecraft/network/protocol/common/ServerboundCustomPayloadPacket"
            }
            MinecraftClassType::ClientboundCustomPayloadPacket => {
                "net/minecraft/network/protocol/common/ClientboundCustomPayloadPacket"
            }
            MinecraftClassType::BrandPayload => {
                "net/minecraft/network/protocol/common/custom/BrandPayload"
            }
            MinecraftClassType::CustomPacketPayload => {
                "net/minecraft/network/protocol/common/custom/CustomPacketPayload"
            }
            MinecraftClassType::ClientboundExplodePacket => {
                "net/minecraft/network/protocol/game/ClientboundExplodePacket"
            }
            MinecraftClassType::ClientboundSystemChatPacket => {
                "net/minecraft/network/protocol/game/ClientboundSystemChatPacket"
            }
            MinecraftClassType::ClientboundPlayerChatPacket => {
                "net/minecraft/network/protocol/game/ClientboundPlayerChatPacket"
            }
            MinecraftClassType::SignedMessageBody => "net/minecraft/network/chat/SignedMessageBody",
            MinecraftClassType::SoundEvent => "net/minecraft/sounds/SoundEvent",
            MinecraftClassType::SoundEvents => "net/minecraft/sounds/SoundEvents",
            MinecraftClassType::SoundSource => "net/minecraft/sounds/SoundSource",
            MinecraftClassType::DustParticleOptions => {
                "net/minecraft/core/particles/DustParticleOptions"
            }
            MinecraftClassType::RenderShape => "net/minecraft/world/level/block/RenderShape",
            MinecraftClassType::BlockEntityRenderDispatcher => {
                "net/minecraft/client/renderer/blockentity/BlockEntityRenderDispatcher"
            }
            MinecraftClassType::BlockRenderDispatcher => {
                "net/minecraft/client/renderer/block/BlockRenderDispatcher"
            }
            MinecraftClassType::ItemInHandRenderer => {
                "net/minecraft/client/renderer/ItemInHandRenderer"
            }
            MinecraftClassType::ScreenEffectRenderer => {
                "net/minecraft/client/renderer/ScreenEffectRenderer"
            }
            MinecraftClassType::ParticleEngine => "net/minecraft/client/particle/ParticleEngine",
        }
    }
}

 
impl fmt::Display for MinecraftClassType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get_name())
    }
}
