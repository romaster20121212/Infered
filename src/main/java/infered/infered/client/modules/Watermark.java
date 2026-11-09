package infered.infered.client.modules;

import infered.infered.client.InferedClient;
import net.minecraft.client.MinecraftClient;
import net.minecraft.client.gui.DrawContext;

 


public class Watermark extends Module {
    public Watermark() {
        super("watermark", "Watermark", "Shows the Infered watermark on the HUD");
    }

    @Override
    public void renderHud(DrawContext context) {
        if (!enabled) return;
        MinecraftClient client = MinecraftClient.getInstance();
        if (client.options.hudHidden) return;

        String name = "Infered";
        String author = " by thefinalblow";
        int nameW = client.textRenderer.getWidth(name);
        int authorW = client.textRenderer.getWidth(author);
        int totalW = nameW + authorW + 12;

         
        int x = 4, y = 4;
        int h = 20;
        
         
        context.fill(x, y, x + totalW, y + h, 0xDD000000);
        context.fill(x, y, x + totalW, y + 1, 0xFFE02020);  
        context.fill(x, y + h - 1, x + totalW, y + h, 0xFFE02020);  
        context.fill(x, y, x + 1, y + h, 0xFFE02020);  
        context.fill(x + totalW - 1, y, x + totalW, y + h, 0xFFE02020);  
        
         
        context.fill(x + 2, y + 2, x + 4, y + h - 2, 0xFFE02020);
        
         
        context.drawText(client.textRenderer, name, x + 8, y + 6, 0xFFE02020, true);
        context.drawText(client.textRenderer, author, x + 8 + nameW, y + 6, 0xFFFFFFFF, true);
    }
}
