package infered.infered.client.modules;

import net.minecraft.client.MinecraftClient;
import net.minecraft.entity.player.PlayerAbilities;
import net.minecraft.util.math.Vec3d;

 



public class FreeCam extends Module {
    private Vec3d originalPos;
    private float originalYaw;
    private float originalPitch;
    private boolean wasFlying;
    private boolean spectatorMode;

    public FreeCam() {
        super("freecam", "FreeCam", "Free camera movement without moving player", false);
    }

    @Override
    public void onEnable() {
        MinecraftClient client = MinecraftClient.getInstance();
        if (client.player == null) return;

         
        originalPos = client.player.getPos();
        originalYaw = client.player.getYaw();
        originalPitch = client.player.getPitch();
        wasFlying = client.player.getAbilities().flying;
        spectatorMode = false;

         
        PlayerAbilities abilities = client.player.getAbilities();
        abilities.allowFlying = true;
        abilities.flying = true;
    }

    @Override
    public void onDisable() {
        MinecraftClient client = MinecraftClient.getInstance();
        if (client.player == null) return;

         
        if (originalPos != null) {
            client.player.setPos(originalPos.x, originalPos.y, originalPos.z);
            client.player.setYaw(originalYaw);
            client.player.setPitch(originalPitch);
        }

         
        PlayerAbilities abilities = client.player.getAbilities();
        abilities.flying = wasFlying;
        if (!wasFlying) {
            abilities.allowFlying = false;
        }

        originalPos = null;
        spectatorMode = false;
    }

    @Override
    public void tick(MinecraftClient client) {
        if (!enabled || client.player == null) return;

         
        PlayerAbilities abilities = client.player.getAbilities();
        if (!abilities.flying) {
            abilities.flying = true;
        }

         
         
    }

    public boolean shouldCancelMovement() {
        return enabled;
    }

    public boolean isSpectatorMode() {
        return spectatorMode;
    }
}