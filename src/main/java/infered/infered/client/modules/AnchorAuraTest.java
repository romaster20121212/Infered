package infered.infered.client.modules;

import net.minecraft.block.Blocks;
import net.minecraft.client.MinecraftClient;
import net.minecraft.entity.Entity;
import net.minecraft.entity.LivingEntity;
import net.minecraft.entity.player.PlayerEntity;
import net.minecraft.item.ItemStack;
import net.minecraft.item.Items;
import net.minecraft.util.Hand;
import net.minecraft.util.hit.BlockHitResult;
import net.minecraft.util.hit.HitResult;
import net.minecraft.util.math.BlockPos;
import net.minecraft.util.math.Direction;
import net.minecraft.util.math.Vec3d;

import java.util.ArrayList;
import java.util.List;
import java.util.Random;

 



public class AnchorAuraTest extends Module {
    private static final double MAX_PLACE_DISTANCE = 4.5;
    private static final double MAX_CHARGE_DISTANCE = 4.5;
    private static final int ANCHOR_MAX_CHARGES = 4;
    private static final double DEFAULT_SELF_DAMAGE = 4.0;
    
    private final Random random = new Random();
    private LivingEntity target = null;
    private BlockPos anchorPos = null;
    private int anchorCharges = 0;
    private int cooldown = 0;
    private int placeCooldown = 0;
    private int chargeCooldown = 0;
    private int detonateCooldown = 0;
    private AnchorState state = AnchorState.IDLE;
    
    private enum AnchorState {
        IDLE,
        PLACING,
        CHARGING,
        DETONATING
    }
    
     
    private boolean autoPlace = true;
    private boolean autoCharge = true;
    private boolean autoDetonate = true;
    private boolean doubleAnchor = false;
    private boolean safeAnchor = false;
    private boolean aimAssist = true;
    private boolean silentAim = true;
    private double aimSpeed = 12.0;
    private int delay = 50;
    private double maxSelfDamage = DEFAULT_SELF_DAMAGE;
    private boolean preventSuicide = true;
    
    public AnchorAuraTest() {
        super("anchor_aura_test", "Anchor Aura Test", "Places, charges, and detonates anchors (Vape port)", false);
    }
    
    @Override
    public void tick(MinecraftClient client) {
        if (!enabled || client.player == null || client.world == null) {
            resetState();
            return;
        }
        
        if (cooldown > 0) {
            cooldown--;
        }
        
        if (placeCooldown > 0) {
            placeCooldown--;
        }
        
        if (chargeCooldown > 0) {
            chargeCooldown--;
        }
        
        if (detonateCooldown > 0) {
            detonateCooldown--;
        }
        
        PlayerEntity player = client.player;
        
         
        updateTarget(client);
        
        if (target == null) {
            resetState();
            return;
        }
        
         
        switch (state) {
            case IDLE:
                if (autoPlace && placeCooldown == 0) {
                    state = AnchorState.PLACING;
                }
                break;
                
            case PLACING:
                if (placeAnchor(client, player)) {
                    state = AnchorState.CHARGING;
                    chargeCooldown = delay;
                } else {
                    state = AnchorState.IDLE;
                    placeCooldown = delay;
                }
                break;
                
            case CHARGING:
                if (chargeCooldown == 0) {
                    if (anchorPos != null && anchorCharges < ANCHOR_MAX_CHARGES) {
                        if (chargeAnchor(client, player)) {
                            anchorCharges++;
                            if (anchorCharges >= ANCHOR_MAX_CHARGES) {
                                state = AnchorState.DETONATING;
                                detonateCooldown = delay;
                            } else {
                                chargeCooldown = delay;
                            }
                        } else {
                            resetState();
                        }
                    } else {
                        state = AnchorState.DETONATING;
                        detonateCooldown = delay;
                    }
                }
                break;
                
            case DETONATING:
                if (detonateCooldown == 0) {
                    if (detonateAnchor(client, player)) {
                        if (doubleAnchor) {
                            state = AnchorState.PLACING;
                            placeCooldown = delay;
                        } else {
                            resetState();
                        }
                    } else {
                        resetState();
                    }
                }
                break;
        }
    }
    
    private void updateTarget(MinecraftClient client) {
        PlayerEntity player = client.player;
        List<LivingEntity> candidates = new ArrayList<>();
        
        for (Entity entity : client.world.getEntities()) {
            if (!(entity instanceof LivingEntity living)) continue;
            if (living == player) continue;
            if (!living.isAlive()) continue;
            if (living.getHealth() <= 0) continue;
            
             
            double distance = player.squaredDistanceTo(entity);
            if (distance > (MAX_PLACE_DISTANCE * MAX_PLACE_DISTANCE)) {
                continue;
            }
            
            candidates.add(living);
        }
        
        if (candidates.isEmpty()) {
            target = null;
            return;
        }
        
         
        candidates.sort((a, b) -> Double.compare(player.squaredDistanceTo(a), player.squaredDistanceTo(b)));
        
        target = candidates.get(0);
    }
    
    private boolean placeAnchor(MinecraftClient client, PlayerEntity player) {
        if (target == null) return false;
        
         
        BlockPos bestPos = findBestAnchorPosition(client, player, target);
        
        if (bestPos == null) return false;
        
         
        int anchorSlot = findAnchorSlot(player);
        if (anchorSlot == -1) return false;
        
         
        if (!isPlacementSafe(client, player, bestPos)) {
            return false;
        }
        
         
        int prevSlot = player.getInventory().selectedSlot;
        if (anchorSlot != prevSlot) {
            player.getInventory().selectedSlot = anchorSlot;
        }
        
         
        Vec3d hitPos = Vec3d.of(bestPos).add(0.5, 0.5, 0.5);
        Direction facing = Direction.UP;
        
        BlockHitResult hitResult = new BlockHitResult(hitPos, facing, bestPos, false);
        
        client.interactionManager.interactBlock((net.minecraft.client.network.ClientPlayerEntity) player, Hand.MAIN_HAND, hitResult);
        player.swingHand(Hand.MAIN_HAND);
        
         
        if (anchorSlot != prevSlot) {
            player.getInventory().selectedSlot = prevSlot;
        }
        
        anchorPos = bestPos;
        anchorCharges = 0;
        placeCooldown = delay;
        return true;
    }
    
    private boolean chargeAnchor(MinecraftClient client, PlayerEntity player) {
        if (anchorPos == null) return false;
        
         
        int glowstoneSlot = findGlowstoneSlot(player);
        if (glowstoneSlot == -1) return false;
        
         
        double distance = player.squaredDistanceTo(Vec3d.ofCenter(anchorPos));
        if (distance > (MAX_CHARGE_DISTANCE * MAX_CHARGE_DISTANCE)) {
            return false;
        }
        
         
        int prevSlot = player.getInventory().selectedSlot;
        if (glowstoneSlot != prevSlot) {
            player.getInventory().selectedSlot = glowstoneSlot;
        }
        
         
        Vec3d hitPos = Vec3d.ofCenter(anchorPos);
        Direction facing = Direction.UP;
        
        BlockHitResult hitResult = new BlockHitResult(hitPos, facing, anchorPos, false);
        
        client.interactionManager.interactBlock((net.minecraft.client.network.ClientPlayerEntity) player, Hand.MAIN_HAND, hitResult);
        player.swingHand(Hand.MAIN_HAND);
        
         
        if (glowstoneSlot != prevSlot) {
            player.getInventory().selectedSlot = prevSlot;
        }
        
        return true;
    }
    
    private boolean detonateAnchor(MinecraftClient client, PlayerEntity player) {
        if (anchorPos == null) return false;
        
         
        var anchorBlock = client.world.getBlockState(anchorPos);
        if (!anchorBlock.isOf(Blocks.RESPAWN_ANCHOR)) {
            resetState();
            return false;
        }
        
         
        double distance = player.squaredDistanceTo(Vec3d.ofCenter(anchorPos));
        if (distance > (MAX_CHARGE_DISTANCE * MAX_CHARGE_DISTANCE)) {
            return false;
        }
        
         
        Vec3d hitPos = Vec3d.ofCenter(anchorPos);
        Direction facing = Direction.UP;
        
        BlockHitResult hitResult = new BlockHitResult(hitPos, facing, anchorPos, false);
        
        client.interactionManager.interactBlock((net.minecraft.client.network.ClientPlayerEntity) player, Hand.MAIN_HAND, hitResult);
        player.swingHand(Hand.MAIN_HAND);
        
        anchorPos = null;
        anchorCharges = 0;
        detonateCooldown = delay;
        return true;
    }
    
    private BlockPos findBestAnchorPosition(MinecraftClient client, PlayerEntity player, LivingEntity target) {
        BlockPos targetPos = target.getBlockPos();
        BlockPos bestPos = null;
        double bestScore = Double.MAX_VALUE;
        
         
        for (int x = -2; x <= 2; x++) {
            for (int y = -1; y <= 2; y++) {
                for (int z = -2; z <= 2; z++) {
                    BlockPos pos = targetPos.add(x, y, z);
                    
                    if (!isValidAnchorPlacement(client, player, pos)) {
                        continue;
                    }
                    
                     
                    double distance = Math.sqrt(pos.getSquaredDistance(targetPos));
                    double score = distance;
                    
                    if (score < bestScore) {
                        bestScore = score;
                        bestPos = pos;
                    }
                }
            }
        }
        
        return bestPos;
    }
    
    private boolean isValidAnchorPlacement(MinecraftClient client, PlayerEntity player, BlockPos pos) {
         
        if (!client.world.getBlockState(pos).isOf(Blocks.OBSIDIAN) && 
            !client.world.getBlockState(pos).isOf(Blocks.BEDROCK) &&
            !client.world.getBlockState(pos).isOf(Blocks.CRYING_OBSIDIAN)) {
            return false;
        }
        
         
        BlockPos above = pos.up();
        if (!client.world.getBlockState(above).isAir()) {
            return false;
        }
        
         
        if (client.world.getBlockState(above).isOf(Blocks.RESPAWN_ANCHOR)) {
            return false;
        }
        
         
        double distance = player.squaredDistanceTo(Vec3d.ofCenter(above));
        if (distance > (MAX_PLACE_DISTANCE * MAX_PLACE_DISTANCE)) {
            return false;
        }
        
        return true;
    }
    
    private boolean isPlacementSafe(MinecraftClient client, PlayerEntity player, BlockPos pos) {
        BlockPos anchorPos = pos.up();
        Vec3d explosionPos = Vec3d.ofCenter(anchorPos);
        
         
        float selfDamage = calculateExplosionDamage(player, explosionPos);
        
        if (preventSuicide && selfDamage >= player.getHealth()) {
            return false;
        }
        
        if (selfDamage > maxSelfDamage) {
            return false;
        }
        
        return true;
    }
    
    private float calculateExplosionDamage(PlayerEntity player, Vec3d explosionPos) {
        double distance = player.getPos().distanceTo(explosionPos);
        
        if (distance > 12.0) {
            return 0.0f;
        }
        
         
        float rawDamage = (float) ((1.0 - distance / 12.0) * 8.0);
        
         
        float armor = player.getArmor();
        if (armor > 0) {
            float effectiveArmor = Math.min(armor, 20.0f);
            rawDamage *= (1.0f - effectiveArmor / 25.0f);
        }
        
        return Math.max(rawDamage, 0.0f);
    }
    
    private int findAnchorSlot(PlayerEntity player) {
        for (int i = 0; i < 9; i++) {
            ItemStack stack = player.getInventory().getStack(i);
            if (stack.getItem() == Items.RESPAWN_ANCHOR) {
                return i;
            }
        }
        return -1;
    }
    
    private int findGlowstoneSlot(PlayerEntity player) {
        for (int i = 0; i < 9; i++) {
            ItemStack stack = player.getInventory().getStack(i);
            if (stack.getItem() == Items.GLOWSTONE) {
                return i;
            }
        }
        return -1;
    }
    
    private void resetState() {
        state = AnchorState.IDLE;
        anchorPos = null;
        anchorCharges = 0;
    }
    
    @Override
    public void onDisable() {
        resetState();
        target = null;
        cooldown = 0;
        placeCooldown = 0;
        chargeCooldown = 0;
        detonateCooldown = 0;
    }
    
     
    public void setAutoPlace(boolean value) { this.autoPlace = value; }
    public void setAutoCharge(boolean value) { this.autoCharge = value; }
    public void setAutoDetonate(boolean value) { this.autoDetonate = value; }
    public void setDoubleAnchor(boolean value) { this.doubleAnchor = value; }
    public void setSafeAnchor(boolean value) { this.safeAnchor = value; }
    public void setAimAssist(boolean value) { this.aimAssist = value; }
    public void setSilentAim(boolean value) { this.silentAim = value; }
    public void setAimSpeed(double value) { this.aimSpeed = value; }
    public void setDelay(int value) { this.delay = value; }
    public void setMaxSelfDamage(double value) { this.maxSelfDamage = value; }
    public void setPreventSuicide(boolean value) { this.preventSuicide = value; }
}
