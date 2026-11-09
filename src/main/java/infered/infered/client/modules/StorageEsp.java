package infered.infered.client.modules;

import infered.infered.client.InferedClientHolder;
import net.minecraft.block.Block;
import net.minecraft.block.BlockState;
import net.minecraft.block.entity.BlockEntity;
import net.minecraft.block.entity.BlockEntityType;
import net.minecraft.registry.Registries;
import net.minecraft.util.math.BlockPos;

import java.util.Set;

 



public class StorageEsp extends ChunkEsp {
     
    public static final Set<String> DEFAULT_BLOCKS = Set.of(
        "minecraft:chest", "minecraft:trapped_chest", "minecraft:barrel", "minecraft:hopper",
        "minecraft:dispenser", "minecraft:dropper", "minecraft:furnace", "minecraft:blast_furnace",
        "minecraft:smoker", "minecraft:hopper_minecart", "minecraft:shulker_box",
        "minecraft:white_shulker_box", "minecraft:orange_shulker_box", "minecraft:magenta_shulker_box",
        "minecraft:light_blue_shulker_box", "minecraft:yellow_shulker_box", "minecraft:lime_shulker_box",
        "minecraft:pink_shulker_box", "minecraft:gray_shulker_box", "minecraft:light_gray_shulker_box",
        "minecraft:cyan_shulker_box", "minecraft:purple_shulker_box", "minecraft:blue_shulker_box",
        "minecraft:brown_shulker_box", "minecraft:green_shulker_box", "minecraft:red_shulker_box",
        "minecraft:black_shulker_box", "minecraft:ender_chest", "minecraft:lectern", "minecraft:jukebox"
    );

    private static final int RED = 0xFFE02020;
    private static final int RED_FILL = 0x60E02020;

    public StorageEsp() {
        super("storageesp", "Storage ESP", "Renders storage blocks through walls (Meteor-style)");
        selectedBlocks.addAll(DEFAULT_BLOCKS);
        storageFilter = StorageEsp::hasInventoryBlockEntity;
    }

    @Override
    protected boolean matches(BlockState state) {
        return selectedBlocks.contains(Registries.BLOCK.getId(state.getBlock()).toString());
    }

    @Override
    protected int getColor() {
        return RED;
    }

    @Override
    protected int getFillColor() {
        return RED_FILL;
    }

     
    public static boolean hasInventoryBlockEntity(Block block) {
        BlockState state = block.getDefaultState();
        for (BlockEntityType<?> type : Registries.BLOCK_ENTITY_TYPE) {
            if (type.supports(state)) {
                try {
                    BlockEntity be = type.instantiate(BlockPos.ORIGIN, state);
                    return be instanceof net.minecraft.inventory.Inventory;
                } catch (Exception e) {
                    return false;
                }
            }
        }
        return false;
    }
}
