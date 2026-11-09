package infered.infered.client.modules;

import infered.infered.client.InferedClientHolder;
import infered.infered.client.render.InferedDrawer;
import net.fabricmc.fabric.api.client.rendering.v1.WorldRenderContext;
import net.minecraft.client.MinecraftClient;
import net.minecraft.entity.Entity;
import net.minecraft.entity.mob.AmbientEntity;
import net.minecraft.entity.mob.Monster;
import net.minecraft.entity.mob.WaterCreatureEntity;
import net.minecraft.entity.passive.AnimalEntity;
import net.minecraft.entity.player.PlayerEntity;
import net.minecraft.util.math.Box;

 





public class EntityEsp extends Module {
     
    private static final int DEFAULT_PLAYERS = 0xFFFFFFFF;
    private static final int DEFAULT_MONSTERS = 0xFFFF1919;
    private static final int DEFAULT_ANIMALS = 0xFF19FF19;
    private static final int DEFAULT_WATER = 0xFF1919FF;
    private static final int DEFAULT_MISC = 0xFFAFAFAF;

     
    private static final double FADE_DISTANCE = 3.0;

    public EntityEsp() {
        super("entityesp", "Entity ESP", "Outlines entities through walls (Meteor-style)");
    }

     

    public enum Category {
        PLAYERS("Players", "entityesp.players", DEFAULT_PLAYERS),
        MONSTERS("Monsters", "entityesp.monsters", DEFAULT_MONSTERS),
        ANIMALS("Animals", "entityesp.animals", DEFAULT_ANIMALS),
        WATER("Water Animals", "entityesp.water", DEFAULT_WATER),
        MISC("Misc", "entityesp.misc", DEFAULT_MISC);

        public final String label;
        public final String settingKey;
        public final int defaultColor;

        Category(String label, String settingKey, int defaultColor) {
            this.label = label;
            this.settingKey = settingKey;
            this.defaultColor = defaultColor;
        }
    }

    public boolean isCategoryOn(Category c) {
         
         
        return InferedClientHolder.config.getSetting(c.settingKey, true);
    }

    public void setCategoryOn(Category c, boolean value) {
        InferedClientHolder.config.setSetting(c.settingKey, value);
    }

    public int getCategoryColor(Category c) {
        return InferedClientHolder.config.getColor(c.settingKey + ".color", c.defaultColor);
    }

    public void setCategoryColor(Category c, int argb) {
        InferedClientHolder.config.setColor(c.settingKey + ".color", argb);
    }

    public boolean isFillOn() {
        return InferedClientHolder.config.getSetting(id + ".fill", false);
    }

    public void setFill(boolean value) {
        InferedClientHolder.config.setSetting(id + ".fill", value);
    }

    @Override
    public void reset() {
        super.reset();
        setFill(false);
        for (Category c : Category.values()) {
            setCategoryOn(c, true);
        }
    }

     

    private Category categorize(Entity e) {
        if (e instanceof PlayerEntity) return Category.PLAYERS;
        if (e instanceof Monster) return Category.MONSTERS;
        if (e instanceof WaterCreatureEntity) return Category.WATER;
        if (e instanceof AnimalEntity || e instanceof AmbientEntity) return Category.ANIMALS;
        return Category.MISC;
    }

     

    @Override
    public void renderLast(WorldRenderContext ctx) {
        if (!enabled) return;
        MinecraftClient client = MinecraftClient.getInstance();
        if (client.world == null || client.player == null) return;

        double fadeSq = FADE_DISTANCE * FADE_DISTANCE;
        boolean fill = isFillOn();

        for (Entity e : client.world.getEntities()) {
             
            if (e == client.player || !e.isAlive()) continue;

            Category category = categorize(e);
            if (!isCategoryOn(category)) continue;

            int color = getCategoryColor(category);
            Box box = e.getBoundingBox();

             
            double distSq = box.getCenter().squaredDistanceTo(ctx.camera().getPos());
            if (distSq < fadeSq) {
                double a = distSq / fadeSq;
                if (a < 0.075) continue;  
                color = scaleAlpha(color, a);
            }

            if (fill) {
                InferedDrawer.drawFilledBox(ctx, box, color);
            }
            InferedDrawer.drawBox(ctx, box, color);
        }
    }

    private static int scaleAlpha(int argb, double scale) {
        int a = (int) ((argb >>> 24) * scale);
        return (a << 24) | (argb & 0xFFFFFF);
    }
}
