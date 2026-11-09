package infered.infered.mixin;

import infered.infered.client.PixelFont;
import net.minecraft.client.MinecraftClient;
import net.minecraft.client.gui.DrawContext;
import net.minecraft.client.render.RenderLayer;
import net.minecraft.util.Identifier;
import net.minecraft.client.gui.screen.SplashOverlay;
import net.minecraft.resource.ResourceReload;
import net.minecraft.util.Util;
import net.minecraft.util.math.MathHelper;
import org.spongepowered.asm.mixin.Final;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Overwrite;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.Unique;

import java.util.Optional;

 







@Mixin(SplashOverlay.class)
public abstract class SplashOverlayMixin {
    @Unique private static final Identifier WORDMARK = Identifier.of("infered", "textures/gui/loading_wordmark.png");
    @Shadow @Final private MinecraftClient client;
    @Shadow @Final private ResourceReload reload;
    @Shadow @Final private java.util.function.Consumer<Optional<Throwable>> exceptionHandler;

    @Unique private static final int RED = 0xFFE02020;
    @Unique private static final int RED_DIM = 0xFF7A1212;
    @Unique private static final int BLACK = 0xFF050505;
    @Unique private static final int BAR_BG = 0xFF1A0A0A;
    @Unique private static final int GRAY = 0xFF9A9A9A;

    @Unique private float infered$smoothed;

     




    @Overwrite
    public void render(DrawContext context, int mouseX, int mouseY, float delta) {
        int w = context.getScaledWindowWidth();
        int h = context.getScaledWindowHeight();

         
         
        float target = reload.getProgress();
        infered$smoothed += (target - infered$smoothed) * Math.min(1f, Math.max(0f, delta) * 6f);
        float p = MathHelper.clamp(infered$smoothed, 0f, 1f);

         
        context.fill(0, 0, w, h, BLACK);
        context.fill(0, 0, w, 2, RED_DIM);
        context.fill(0, h - 2, w, h, RED_DIM);

         
         
        int logoW = Math.min(320, w - 40);
        int logoH = logoW * 171 / 512;
        context.drawGuiTexture(RenderLayer::getGuiTextured, WORDMARK, (w - logoW) / 2, h / 2 - logoH - 6, logoW, logoH);

         
        int aw = PixelFont.width("by thefinalblow", 1);
        PixelFont.draw(context, "by thefinalblow", (w - aw) / 2, h / 2 - 8, GRAY, 1);

         
        int barW = Math.min(240, w / 2);
        int barX = (w - barW) / 2;
        int barY = h / 2 + 14;
        context.fill(barX - 1, barY - 1, barX + barW + 1, barY + 11, RED_DIM);
        context.fill(barX, barY, barX + barW, barY + 10, BAR_BG);
        int fill = (int) (barW * p);
        if (fill > 0) {
            context.fill(barX, barY, barX + fill, barY + 10, RED);
        }

         
        String pct = (int) (p * 100) + "%";
        int pw = PixelFont.width(pct, 1);
        PixelFont.draw(context, pct, (w - pw) / 2, barY + 16, GRAY, 1);

         
         
         
         
         
        if (reload.isComplete()) {
            try {
                reload.throwException();
                exceptionHandler.accept(Optional.empty());
            } catch (Throwable t) {
                exceptionHandler.accept(Optional.of(t));
            }
            client.setOverlay(null);
        }
    }
}
