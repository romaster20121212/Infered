package infered.infered.client.modules;

import infered.infered.client.InferedClientHolder;
import infered.infered.client.menu.EspSettingsScreen;
import net.minecraft.client.MinecraftClient;
import net.minecraft.client.gui.DrawContext;
import net.minecraft.client.gui.screen.Screen;

 


public abstract class Module {
    public final String id;
    public final String name;
    public final String description;
    public final boolean hasSettings;
    public boolean enabled;

    public Module(String id, String name, String description) {
        this(id, name, description, false);
    }

    public Module(String id, String name, String description, boolean hasSettings) {
        this.id = id;
        this.name = name;
        this.description = description;
        this.hasSettings = hasSettings;
    }

    public String getId() { return id; }
    public String getName() { return name; }
    public String getDescription() { return description; }
    public boolean isEnabled() { return enabled; }

    public void setEnabled(boolean value) {
        if (enabled == value) return;
        enabled = value;
        if (enabled) onEnable(); else onDisable();
    }

    public void toggle() {
        setEnabled(!enabled);
    }

     
    public Screen createSettingsScreen() {
        return new EspSettingsScreen(this);
    }

     
    public void reset() {
    }

    public void onEnable() {}
    public void onDisable() {}
    public void tick(MinecraftClient client) {}
    public void renderHud(DrawContext context) {}

     



    public void renderLast(net.fabricmc.fabric.api.client.rendering.v1.WorldRenderContext ctx) {}
}
