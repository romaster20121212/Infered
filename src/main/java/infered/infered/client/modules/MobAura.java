package infered.infered.client.modules;

import net.minecraft.client.MinecraftClient;
import net.minecraft.entity.Entity;
import net.minecraft.entity.LivingEntity;
import net.minecraft.entity.mob.MobEntity;
import net.minecraft.entity.player.PlayerEntity;
import net.minecraft.util.Hand;

 


public class MobAura extends Module {
    private int cooldown = 0;
    private static final double RANGE = 4.5;
    private static final int ATTACK_COOLDOWN = 10;  

    public MobAura() {
        super("mobaura", "MobAura", "Automatically attacks hostile mobs", false);
    }

    @Override
    public void tick(MinecraftClient client) {
        if (!enabled || client.player == null || client.world == null) return;
        
        if (cooldown > 0) {
            cooldown--;
            return;
        }

        PlayerEntity player = client.player;
        
         
        LivingEntity target = findClosestMob(player, client);
        if (target == null) return;
        
         
        boolean shouldAttack = true;
        Criticals criticals = ModuleManager.get().criticals;
        if (criticals != null && criticals.areEnabled()) {
            shouldAttack = criticals.prepareAttack(client);
        }
        
        if (shouldAttack) {
             
            client.interactionManager.attackEntity(player, target);
            player.swingHand(Hand.MAIN_HAND);
            cooldown = ATTACK_COOLDOWN;
        }
    }

    private LivingEntity findClosestMob(PlayerEntity player, MinecraftClient client) {
        LivingEntity closest = null;
        double closestDist = RANGE;
        
        for (Entity entity : client.world.getEntities()) {
            if (!(entity instanceof MobEntity)) continue;
            if (!(entity instanceof LivingEntity living)) continue;
            
             
            if (!living.isAlive() || living == player) continue;
            
             
            double dist = player.squaredDistanceTo(entity);
            if (dist < closestDist * closestDist) {
                closest = living;
                closestDist = Math.sqrt(dist);
            }
        }
        
        return closest;
    }
}