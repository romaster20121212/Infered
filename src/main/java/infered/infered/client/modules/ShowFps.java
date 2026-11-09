package infered.infered.client.modules;

import net.minecraft.client.MinecraftClient;
import net.minecraft.client.gui.DrawContext;

 


public class ShowFps extends Module {
    public ShowFps() {
        super("showfps", "Show FPS", "Displays your current FPS on the HUD");
    }

    @Override
    public void renderHud(DrawContext context) {
        if (!enabled) return;
        MinecraftClient client = MinecraftClient.getInstance();
        if (client.options.hudHidden || client.world == null) return;

        String fps = client.getCurrentFps() + " FPS";
        int w = client.textRenderer.getWidth(fps);
        int x = context.getScaledWindowWidth() - w - 4;
         
        context.fill(x - 3, 4, x + w + 3, 16, 0x90000000);
        context.drawText(client.textRenderer, fps, x, 7, 0xFFE02020, true);
    }
}
