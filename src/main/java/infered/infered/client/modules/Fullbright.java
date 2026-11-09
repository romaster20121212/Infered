package infered.infered.client.modules;

import infered.infered.client.InferedClientHolder;
import net.minecraft.client.MinecraftClient;

 









public class Fullbright extends Module {
    public Fullbright() {
        super("fullbright", "Fullbright", "Fully brightens the world (no darkness)", false);
    }

     
    public static boolean isForcing() {
        return InferedClientHolder.manager.fullbright.enabled;
    }

    @Override
    public void tick(MinecraftClient client) {
         
    }
}
