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

 



public class CrystalFlowTest extends Module {
    private static final double MAX_PLACE_DISTANCE = 4.5;
    private static final double MAX_BREAK_DISTANCE = 4.5;
    private static final int DEFAULT_CRYSTAL_DAMAGE = 6;
    private static final double MAX_SELF_DAMAGE = 4.0;
    
    private final Random random = new Random();
    private LivingEntity target = null;
    private int cooldown = 0;
    private int attackCooldown = 0;
    private int placeCooldown = 0;
    
     
    private boolean autoPlace = true;
    private boolean autoBreak = true;
    private double placeRange = MAX_PLACE_DISTANCE;
    private double breakRange = MAX_BREAK_DISTANCE;
    private double maxSelfDamage = MAX_SELF_DAMAGE;
    private boolean preventSuicide = true;
    private boolean antiSuicide = true;
    private boolean checkArmor = true;
    private boolean antiTotem = false;
    private int minEnchantArmor = 10;
    private int switchDelay = 3;
    private int placeDelay = 2;
    private int breakDelay = 2;
    
    public CrystalFlowTest() {
        super("crystal_flow_test", "Crystal Flow Test", "Auto-places and breaks end crystals (Vape port)", false);
    }
    
    @Override
    public void tick(MinecraftClient client) {
        if (!enabled || client.player == null || client.world == null) {
            target = null;
            return;
        }
        
        if (cooldown > 0) {
            cooldown--;
        }
        
        if (attackCooldown > 0) {
            attackCooldown--;
        }
        
        if (placeCooldown > 0) {
            placeCooldown--;
        }
        
        PlayerEntity player = client.player;
        
         
        updateTarget(client);
        
        if (target == null) {
            return;
        }
        
         
        if (autoBreak && attackCooldown == 0) {
            breakNearbyCrystals(client, player);
        }
        
         
        if (autoPlace && placeCooldown == 0) {
            placeCrystal(client, player);
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
    
    private void breakNearbyCrystals(MinecraftClient client, PlayerEntity player) {
         
        for (Entity entity : client.world.getEntities()) {
            if (!(entity instanceof net.minecraft.entity.decoration.EndCrystalEntity crystal)) continue;
            
            double distance = player.squaredDistanceTo(entity);
            if (distance > (breakRange * breakRange)) {
                continue;
            }
            
             
            if (target != null && player.squaredDistanceTo(target) > (breakRange * breakRange)) {
                double crystalToTarget = crystal.squaredDistanceTo(target);
                if (crystalToTarget > 9.0) {  
                    continue;
                }
            }
            
             
            client.interactionManager.attackEntity(player, crystal);
            player.swingHand(Hand.MAIN_HAND);
            attackCooldown = breakDelay;
            return;
        }
    }
    
    private void placeCrystal(MinecraftClient client, PlayerEntity player) {
        if (target == null) return;
        
         
        BlockPos bestPos = findBestPlacementPosition(client, player, target);
        
        if (bestPos == null) return;
        
         
        int crystalSlot = findCrystalSlot(player);
        if (crystalSlot == -1) return;
        
         
        if (!isPlacementSafe(client, player, bestPos)) {
            return;
        }
        
         
        int prevSlot = player.getInventory().selectedSlot;
        if (crystalSlot != prevSlot) {
            player.getInventory().selectedSlot = crystalSlot;
        }
        
         
        Vec3d hitPos = Vec3d.of(bestPos).add(0.5, 0.5, 0.5);
        Direction facing = Direction.UP;
        
        BlockHitResult hitResult = new BlockHitResult(hitPos, facing, bestPos, false);
        
        client.interactionManager.interactBlock((net.minecraft.client.network.ClientPlayerEntity) player, Hand.MAIN_HAND, hitResult);
        player.swingHand(Hand.MAIN_HAND);
        
         
        if (crystalSlot != prevSlot) {
            player.getInventory().selectedSlot = prevSlot;
        }
        
        placeCooldown = placeDelay;
    }
    
    private BlockPos findBestPlacementPosition(MinecraftClient client, PlayerEntity player, LivingEntity target) {
        BlockPos targetPos = target.getBlockPos();
        BlockPos bestPos = null;
        double bestScore = Double.MAX_VALUE;
        
         
        for (int x = -2; x <= 2; x++) {
            for (int y = -1; y <= 2; y++) {
                for (int z = -2; z <= 2; z++) {
                    BlockPos pos = targetPos.add(x, y, z);
                    
                    if (!isValidPlacement(client, player, pos)) {
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
    
    private boolean isValidPlacement(MinecraftClient client, PlayerEntity player, BlockPos pos) {
         
        if (!client.world.getBlockState(pos).isOf(Blocks.OBSIDIAN) && 
            !client.world.getBlockState(pos).isOf(Blocks.BEDROCK) &&
            !client.world.getBlockState(pos).isOf(Blocks.END_STONE)) {
            return false;
        }
        
         
        BlockPos above = pos.up();
        if (!client.world.getBlockState(above).isAir()) {
            return false;
        }
        
         
        for (Entity entity : client.world.getEntities()) {
            if (entity instanceof net.minecraft.entity.decoration.EndCrystalEntity) {
                if (entity.getBlockPos().equals(above)) {
                    return false;
                }
            }
        }
        
         
        double distance = player.squaredDistanceTo(Vec3d.ofCenter(above));
        if (distance > (placeRange * placeRange)) {
            return false;
        }
        
        return true;
    }
    
    private boolean isPlacementSafe(MinecraftClient client, PlayerEntity player, BlockPos pos) {
        BlockPos crystalPos = pos.up();
        Vec3d explosionPos = Vec3d.ofCenter(crystalPos);
        
         
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
        
         
        float rawDamage = (float) ((1.0 - distance / 12.0) * DEFAULT_CRYSTAL_DAMAGE);
        
         
        float armor = player.getArmor();
        if (armor > 0) {
            float effectiveArmor = Math.min(armor, 20.0f);
            rawDamage *= (1.0f - effectiveArmor / 25.0f);
        }
        
        return Math.max(rawDamage, 0.0f);
    }
    
    private int findCrystalSlot(PlayerEntity player) {
        for (int i = 0; i < 9; i++) {
            ItemStack stack = player.getInventory().getStack(i);
            if (stack.getItem() == Items.END_CRYSTAL) {
                return i;
            }
        }
        return -1;
    }
    
    @Override
    public void onDisable() {
        target = null;
        cooldown = 0;
        attackCooldown = 0;
        placeCooldown = 0;
    }
    
     
    public void setAutoPlace(boolean value) { this.autoPlace = value; }
    public void setAutoBreak(boolean value) { this.autoBreak = value; }
    public void setPlaceRange(double value) { this.placeRange = value; }
    public void setBreakRange(double value) { this.breakRange = value; }
    public void setMaxSelfDamage(double value) { this.maxSelfDamage = value; }
    public void setPreventSuicide(boolean value) { this.preventSuicide = value; }
    public void setAntiSuicide(boolean value) { this.antiSuicide = value; }
    public void setCheckArmor(boolean value) { this.checkArmor = value; }
    public void setAntiTotem(boolean value) { this.antiTotem = value; }
    public void setMinEnchantArmor(int value) { this.minEnchantArmor = value; }
    public void setSwitchDelay(int value) { this.switchDelay = value; }
    public void setPlaceDelay(int value) { this.placeDelay = value; }
    public void setBreakDelay(int value) { this.breakDelay = value; }
}
