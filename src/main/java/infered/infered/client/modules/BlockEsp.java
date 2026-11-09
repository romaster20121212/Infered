package infered.infered.client.modules;

import net.minecraft.block.BlockState;

import java.util.Set;

 




public class BlockEsp extends ChunkEsp {
     
    public static final Set<String> DEFAULT_BLOCKS = Set.of(
        "minecraft:chest", "minecraft:trapped_chest", "minecraft:barrel",
        "minecraft:ancient_debris", "minecraft:diamond_ore", "minecraft:deepslate_diamond_ore",
        "minecraft:emerald_ore", "minecraft:deepslate_emerald_ore", "minecraft:gold_ore",
        "minecraft:deepslate_gold_ore", "minecraft:iron_ore", "minecraft:deepslate_iron_ore",
        "minecraft:spawner", "minecraft:end_portal_frame"
    );

    private static final int RED = 0xFFE02020;
    private static final int RED_FILL = 0x60E02020;

    public BlockEsp() {
        super("blockesp", "Block ESP", "Renders selected blocks through walls (Meteor-style)");
        selectedBlocks.addAll(DEFAULT_BLOCKS);
    }

    @Override
    protected boolean matches(BlockState state) {
        return selectedBlocks.contains(net.minecraft.registry.Registries.BLOCK.getId(state.getBlock()).toString());
    }

    @Override
    protected int getColor() {
        return RED;
    }

    @Override
    protected int getFillColor() {
        return RED_FILL;
    }
}
