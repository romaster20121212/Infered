package infered.infered.client.modules;

import net.minecraft.client.MinecraftClient;

 



public class Velocity extends Module {
    private float horizontal = 0.0f;  
    private float vertical = 0.0f;    

    public Velocity() {
        super("velocity", "Velocity", "Reduces or cancels knockback", true);
    }

    @Override
    public void tick(MinecraftClient client) {
         
    }

     


    public void setHorizontal(float value) {
        this.horizontal = Math.max(0.0f, Math.min(1.0f, value));
    }

     


    public void setVertical(float value) {
        this.vertical = Math.max(0.0f, Math.min(1.0f, value));
    }

     


    public float getHorizontal() {
        return horizontal;
    }

     


    public float getVertical() {
        return vertical;
    }

    @Override
    public void reset() {
        horizontal = 0.0f;
        vertical = 0.0f;
    }
}