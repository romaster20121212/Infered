package infered.infered.client.modules;

import infered.infered.client.InferedClientHolder;
import infered.infered.client.menu.TestModule676767SettingsScreen;
import net.minecraft.block.Blocks;
import net.minecraft.client.MinecraftClient;
import net.minecraft.client.network.ClientPlayerEntity;
import net.minecraft.entity.player.PlayerEntity;
import net.minecraft.item.Items;
import net.minecraft.util.ActionResult;
import net.minecraft.util.Hand;
import net.minecraft.util.hit.BlockHitResult;
import net.minecraft.util.math.BlockPos;
import net.minecraft.util.math.Direction;
import net.minecraft.util.math.Vec3d;
import net.minecraft.world.World;

 




public final class TestModule676767 extends Module {
    private enum Phase {
        IDLE,
        WAIT_BEFORE_OBSIDIAN_SWITCH,
        WAIT_BEFORE_OBSIDIAN_PLACE,
        VERIFY_OBSIDIAN,
        WAIT_BEFORE_CRYSTAL_SWITCH,
        WAIT_BEFORE_CRYSTAL_PLACE
    }

    private Phase phase = Phase.IDLE;
    private PlayerEntity sequencePlayer;
    private World sequenceWorld;
    private Hand crystalHand;
    private BlockHitResult originalHit;
    private BlockPos obsidianPos;
    private int originalSlot = -1;
    private int obsidianSlot = -1;
    private int expectedSelectedSlot = -1;
    private int ticksRemaining;
    private int verifyTicksRemaining;
    private int cooldown;
    private boolean runningSyntheticInteraction;

    private int switchToObsidianDelay = 0;
    private int obsidianPlaceDelay = 0;
    private int verificationDelay = 2;
    private int confirmationTimeout = 12;
    private int switchToCrystalDelay = 0;
    private int crystalPlaceDelay = 1;
    private int cooldownTicks = 2;
    private boolean verifyPlacement = true;
    private boolean restoreSlotOnFailure = true;
    private boolean skipExistingCrystalBases = true;

    public TestModule676767() {
        super("test_module_67_67_67", "Test Module 67 67 67",
            "Places obsidian, confirms it, then places an end crystal there", true);
    }

    @Override
    public net.minecraft.client.gui.screen.Screen createSettingsScreen() {
        return new TestModule676767SettingsScreen(this, MinecraftClient.getInstance().currentScreen);
    }

     



    public ActionResult onUseBlock(PlayerEntity player, World world, Hand hand, BlockHitResult hit) {
        if (runningSyntheticInteraction || !enabled) return ActionResult.PASS;

        MinecraftClient client = MinecraftClient.getInstance();
        if (player != client.player || world != client.world || client.interactionManager == null) {
            return ActionResult.PASS;
        }

        if (phase != Phase.IDLE) {
             
             
            return ActionResult.FAIL;
        }
        if (cooldown > 0 || !player.getStackInHand(hand).isOf(Items.END_CRYSTAL)) {
            return ActionResult.PASS;
        }

        BlockPos clickedPos = hit.getBlockPos();
        if (skipExistingCrystalBases) {
            var clickedState = world.getBlockState(clickedPos);
            if (clickedState.isOf(Blocks.OBSIDIAN) || clickedState.isOf(Blocks.BEDROCK)) {
                return ActionResult.PASS;
            }
        }

        int foundObsidianSlot = findObsidianSlot(player);
        if (foundObsidianSlot < 0) return ActionResult.PASS;

        BlockPos destination = world.getBlockState(clickedPos).isReplaceable()
            ? clickedPos
            : clickedPos.offset(hit.getSide());

         
         
        if (!world.getBlockState(destination).isReplaceable()) return ActionResult.PASS;

        this.sequencePlayer = player;
        this.sequenceWorld = world;
        this.crystalHand = hand;
        this.originalHit = hit;
        this.obsidianPos = destination;
        this.originalSlot = player.getInventory().selectedSlot;
        this.obsidianSlot = foundObsidianSlot;
        this.expectedSelectedSlot = originalSlot;
        this.ticksRemaining = switchToObsidianDelay;
        this.phase = Phase.WAIT_BEFORE_OBSIDIAN_SWITCH;
        return ActionResult.SUCCESS;
    }

    @Override
    public void tick(MinecraftClient client) {
        if (cooldown > 0) cooldown--;
        if (phase == Phase.IDLE) return;

        if (!enabled || client.player != sequencePlayer || client.world != sequenceWorld
            || sequencePlayer == null || sequenceWorld == null
            || sequencePlayer.getInventory().selectedSlot != expectedSelectedSlot) {
            cancel(client, true);
            return;
        }

        switch (phase) {
            case WAIT_BEFORE_OBSIDIAN_SWITCH -> {
                if (countDown()) return;
                if (!sequencePlayer.getInventory().getStack(obsidianSlot).isOf(Items.OBSIDIAN)) {
                    cancel(client, true);
                    return;
                }
                sequencePlayer.getInventory().selectedSlot = obsidianSlot;
                expectedSelectedSlot = obsidianSlot;
                phase = Phase.WAIT_BEFORE_OBSIDIAN_PLACE;
                ticksRemaining = obsidianPlaceDelay;
            }
            case WAIT_BEFORE_OBSIDIAN_PLACE -> {
                if (countDown()) return;
                if (!sequencePlayer.getInventory().getStack(obsidianSlot).isOf(Items.OBSIDIAN)
                    || !sequenceWorld.getBlockState(obsidianPos).isReplaceable()) {
                    cancel(client, true);
                    return;
                }
                ActionResult result = interact(client, Hand.MAIN_HAND, originalHit);
                if (!result.isAccepted()) {
                    cancel(client, true);
                    return;
                }
                verifyTicksRemaining = confirmationTimeout;
                ticksRemaining = verificationDelay;
                phase = Phase.VERIFY_OBSIDIAN;
            }
            case VERIFY_OBSIDIAN -> {
                if (ticksRemaining > 0) {
                    ticksRemaining--;
                    return;
                }
                if (sequenceWorld.getBlockState(obsidianPos).isOf(Blocks.OBSIDIAN) || !verifyPlacement) {
                    sequencePlayer.getInventory().selectedSlot = originalSlot;
                    expectedSelectedSlot = originalSlot;
                    phase = Phase.WAIT_BEFORE_CRYSTAL_SWITCH;
                    ticksRemaining = switchToCrystalDelay;
                    return;
                }
                if (--verifyTicksRemaining <= 0) {
                    cancel(client, true);
                }
            }
            case WAIT_BEFORE_CRYSTAL_SWITCH -> {
                if (countDown()) return;
                 
                 
                phase = Phase.WAIT_BEFORE_CRYSTAL_PLACE;
                ticksRemaining = crystalPlaceDelay;
            }
            case WAIT_BEFORE_CRYSTAL_PLACE -> {
                if (countDown()) return;
                if (!sequencePlayer.getStackInHand(crystalHand).isOf(Items.END_CRYSTAL)
                    || (verifyPlacement && !sequenceWorld.getBlockState(obsidianPos).isOf(Blocks.OBSIDIAN))) {
                    cancel(client, true);
                    return;
                }
                BlockHitResult crystalHit = new BlockHitResult(
                    Vec3d.ofCenter(obsidianPos).add(0.0, 0.5, 0.0),
                    Direction.UP,
                    obsidianPos,
                    false
                );
                ActionResult result = interact(client, crystalHand, crystalHit);
                if (result.isAccepted()) {
                    finish();
                } else {
                    cancel(client, true);
                }
            }
            case IDLE -> { }
        }
    }

    private boolean countDown() {
        if (ticksRemaining > 0) {
            ticksRemaining--;
            return true;
        }
        return false;
    }

    private ActionResult interact(MinecraftClient client, Hand hand, BlockHitResult hit) {
        if (client.interactionManager == null || !(sequencePlayer instanceof ClientPlayerEntity clientPlayer)) {
            return ActionResult.FAIL;
        }
        runningSyntheticInteraction = true;
        try {
            ActionResult result = client.interactionManager.interactBlock(clientPlayer, hand, hit);
            if (result.isAccepted()) sequencePlayer.swingHand(hand);
            return result;
        } finally {
            runningSyntheticInteraction = false;
        }
    }

    private int findObsidianSlot(PlayerEntity player) {
        for (int slot = 0; slot < 9; slot++) {
            if (player.getInventory().getStack(slot).isOf(Items.OBSIDIAN)) return slot;
        }
        return -1;
    }

    private void cancel(MinecraftClient client, boolean failed) {
        if (failed && restoreSlotOnFailure && sequencePlayer != null
            && client.player == sequencePlayer
            && sequencePlayer.getInventory().selectedSlot == expectedSelectedSlot
            && originalSlot >= 0) {
            sequencePlayer.getInventory().selectedSlot = originalSlot;
        }
        finish();
    }

    private void finish() {
        phase = Phase.IDLE;
        sequencePlayer = null;
        sequenceWorld = null;
        crystalHand = null;
        originalHit = null;
        obsidianPos = null;
        originalSlot = -1;
        obsidianSlot = -1;
        expectedSelectedSlot = -1;
        ticksRemaining = 0;
        verifyTicksRemaining = 0;
        cooldown = Math.max(0, cooldownTicks);
    }

    @Override
    public void onDisable() {
        cancel(MinecraftClient.getInstance(), true);
        cooldown = 0;
    }

     
    public void loadSettings() {
        var config = InferedClientHolder.config;
        switchToObsidianDelay = clamp(config.getInt(id + ".switch_to_obsidian", 0), 0, 40);
        obsidianPlaceDelay = clamp(config.getInt(id + ".place_obsidian", 0), 0, 40);
        verificationDelay = clamp(config.getInt(id + ".verify_delay", 2), 0, 40);
        confirmationTimeout = clamp(config.getInt(id + ".verify_timeout", 12), 1, 80);
        switchToCrystalDelay = clamp(config.getInt(id + ".switch_to_crystal", 0), 0, 40);
        crystalPlaceDelay = clamp(config.getInt(id + ".place_crystal", 1), 0, 40);
        cooldownTicks = clamp(config.getInt(id + ".cooldown", 2), 0, 80);
        verifyPlacement = config.getSetting(id + ".verify_placement", true);
        restoreSlotOnFailure = config.getSetting(id + ".restore_on_failure", true);
        skipExistingCrystalBases = config.getSetting(id + ".skip_existing_bases", true);
    }

    public int getSwitchToObsidianDelay() { return switchToObsidianDelay; }
    public void setSwitchToObsidianDelay(int value) { switchToObsidianDelay = clamp(value, 0, 40); saveInt("switch_to_obsidian", switchToObsidianDelay); }
    public int getObsidianPlaceDelay() { return obsidianPlaceDelay; }
    public void setObsidianPlaceDelay(int value) { obsidianPlaceDelay = clamp(value, 0, 40); saveInt("place_obsidian", obsidianPlaceDelay); }
    public int getVerificationDelay() { return verificationDelay; }
    public void setVerificationDelay(int value) { verificationDelay = clamp(value, 0, 40); saveInt("verify_delay", verificationDelay); }
    public int getConfirmationTimeout() { return confirmationTimeout; }
    public void setConfirmationTimeout(int value) { confirmationTimeout = clamp(value, 1, 80); saveInt("verify_timeout", confirmationTimeout); }
    public int getSwitchToCrystalDelay() { return switchToCrystalDelay; }
    public void setSwitchToCrystalDelay(int value) { switchToCrystalDelay = clamp(value, 0, 40); saveInt("switch_to_crystal", switchToCrystalDelay); }
    public int getCrystalPlaceDelay() { return crystalPlaceDelay; }
    public void setCrystalPlaceDelay(int value) { crystalPlaceDelay = clamp(value, 0, 40); saveInt("place_crystal", crystalPlaceDelay); }
    public int getCooldownTicks() { return cooldownTicks; }
    public void setCooldownTicks(int value) { cooldownTicks = clamp(value, 0, 80); saveInt("cooldown", cooldownTicks); }
    public boolean isVerifyPlacement() { return verifyPlacement; }
    public void setVerifyPlacement(boolean value) { verifyPlacement = value; saveBoolean("verify_placement", value); }
    public boolean isRestoreSlotOnFailure() { return restoreSlotOnFailure; }
    public void setRestoreSlotOnFailure(boolean value) { restoreSlotOnFailure = value; saveBoolean("restore_on_failure", value); }
    public boolean isSkipExistingCrystalBases() { return skipExistingCrystalBases; }
    public void setSkipExistingCrystalBases(boolean value) { skipExistingCrystalBases = value; saveBoolean("skip_existing_bases", value); }

    private void saveInt(String key, int value) {
        if (InferedClientHolder.config != null) InferedClientHolder.config.setInt(id + "." + key, value);
    }

    private void saveBoolean(String key, boolean value) {
        if (InferedClientHolder.config != null) InferedClientHolder.config.setSetting(id + "." + key, value);
    }

    private static int clamp(int value, int min, int max) {
        return Math.max(min, Math.min(max, value));
    }
}
