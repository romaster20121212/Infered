package infered.infered.client.modules;

import net.minecraft.client.MinecraftClient;
import net.minecraft.entity.LivingEntity;

 



public class Criticals extends Module {
    private int cooldown = 0;

    public Criticals() {
        super("criticals", "Criticals", "Makes attacks land critical hits", false);
    }

    @Override
    public void tick(MinecraftClient client) {
        if (!enabled || client.player == null) return;
        
        if (cooldown > 0) {
            cooldown--;
        }
    }

     



    public boolean prepareAttack(MinecraftClient client) {
        if (!enabled || client.player == null) return true;
        
        LivingEntity player = client.player;
        
         
        if (!player.isOnGround() && player.fallDistance > 0) {
            return true;
        }
        
         
        if (player.isOnGround()) {
             
            player.setVelocity(player.getVelocity().x, 0.42, player.getVelocity().z);
            cooldown = 5;  
            return false;  
        }
        
         
        return false;
    }

     


    public boolean areEnabled() {
        return enabled;
    }
}