package infered.infered.client.menu;

import infered.infered.client.InferedClient;
import infered.infered.client.InferedClientHolder;
import infered.infered.client.modules.Module;
import infered.infered.client.modules.ModuleManager;
import net.minecraft.client.MinecraftClient;
import net.minecraft.client.gui.DrawContext;
import net.minecraft.client.gui.screen.Screen;
import net.minecraft.client.sound.PositionedSoundInstance;
import net.minecraft.sound.SoundEvents;
import net.minecraft.text.Text;
import net.minecraft.util.Util;
import org.lwjgl.glfw.GLFW;

import java.util.List;

 




public class ModMenu extends Screen {
    private static final int RED = 0xFFE02020;
    private static final int RED_DIM = 0xFF7A1212;
    private static final int HEADER = 0xF0141414;
    private static final int ROW_BG = 0xB40E0E0E;
    private static final int ROW_BG_ALT = 0xB4121212;
    private static final int TINT = 0x600A0A0A;  
    private static final int TEXT_WHITE = 0xFFF2F2F2;
    private static final int TEXT_GRAY = 0xFF8A8A8A;

    private static final long OPEN_MS = 200;
    private static final long CLOSE_MS = 160;
    private static final long HOVER_MS = 120;
    private static final int ROW_H = 22;

    private enum State { OPENING, OPEN, CLOSING }

    private State state = State.OPENING;
    private long animStart;
    private final List<Module> modules;
    private int selectedIndex;

     
    private long[] hoverStart;

     
    private int winX, winY, winW = 220, headerH = 26;

    public ModMenu() {
        super(Text.literal("Infered"));
        this.modules = ModuleManager.get().getModules();
        this.hoverStart = new long[modules.size()];
        java.util.Arrays.fill(hoverStart, -1L);
    }

    @Override
    protected void init() {
        animStart = Util.getMeasuringTimeMs();
        winX = 20;
        winY = 40;
    }

    private float progress(long now, long start, long duration) {
        return Math.min(1f, Math.max(0f, (now - start) / (float) duration));
    }

    private float easeOut(float t) {
        return 1f - (1f - t) * (1f - t);
    }

    @Override
    public void close() {
        if (state != State.CLOSING) {
            state = State.CLOSING;
            animStart = Util.getMeasuringTimeMs();
        }
    }

    @Override
    public void tick() {
        if (state == State.CLOSING && Util.getMeasuringTimeMs() - animStart >= CLOSE_MS) {
            MinecraftClient.getInstance().setScreen(null);
        }
    }

    @Override
    public boolean shouldPause() {
        return false;
    }

     

    @Override
    public boolean keyPressed(int keyCode, int scanCode, int modifiers) {
        switch (keyCode) {
            case GLFW.GLFW_KEY_ESCAPE -> {
                close();
                return true;
            }
            case GLFW.GLFW_KEY_TAB -> {
                selectedIndex = (selectedIndex + 1) % modules.size();
                return true;
            }
            case GLFW.GLFW_KEY_UP -> {
                selectedIndex = (selectedIndex - 1 + modules.size()) % modules.size();
                return true;
            }
            case GLFW.GLFW_KEY_DOWN -> {
                selectedIndex = (selectedIndex + 1) % modules.size();
                return true;
            }
            case GLFW.GLFW_KEY_ENTER, GLFW.GLFW_KEY_RIGHT, GLFW.GLFW_KEY_LEFT -> {
                toggleIndex(selectedIndex);
                return true;
            }
        }
        return super.keyPressed(keyCode, scanCode, modifiers);
    }

    @Override
    public boolean mouseClicked(double mx, double my, int button) {
        if (button == 0) {
            for (int i = 0; i < modules.size(); i++) {
                int ry = winY + headerH + i * ROW_H;
                if (mx >= winX && mx < winX + winW && my >= ry && my < ry + ROW_H) {
                    selectedIndex = i;
                    toggleIndex(i);
                    return true;
                }
            }
        } else if (button == 1) {
             
            for (int i = 0; i < modules.size(); i++) {
                Module m = modules.get(i);
                int ry = winY + headerH + i * ROW_H;
                if (m.hasSettings && mx >= winX && mx < winX + winW && my >= ry && my < ry + ROW_H) {
                    selectedIndex = i;
                    MinecraftClient.getInstance().getSoundManager()
                        .play(PositionedSoundInstance.master(SoundEvents.UI_BUTTON_CLICK, 1.0f));
                    MinecraftClient.getInstance().setScreen(m.createSettingsScreen());
                    return true;
                }
            }
        }
        return super.mouseClicked(mx, my, button);
    }

    private void toggleIndex(int i) {
        Module m = modules.get(i);
        m.toggle();
        InferedClientHolder.config.setEnabled(m.getId(), m.isEnabled());
        MinecraftClient.getInstance().getSoundManager()
            .play(PositionedSoundInstance.master(SoundEvents.UI_BUTTON_CLICK, m.isEnabled() ? 1.1f : 0.8f));
    }

     

    @Override
    public void render(DrawContext ctx, int mouseX, int mouseY, float delta) {
        long now = Util.getMeasuringTimeMs();
        boolean closing = state == State.CLOSING;
        float p = easeOut(progress(now, animStart, closing ? CLOSE_MS : OPEN_MS));
        float move = closing ? 1f - p : p;

         
        ctx.fill(0, 0, width, height, TINT);

        int winH = headerH + modules.size() * ROW_H;
         
        int x = winX - (int) ((1f - move) * (winW + 30));
        int alpha = (int) (255 * move);
        int a = alpha;

         
        ctx.fill(x, winY, x + winW, winY + headerH, withAlpha(HEADER, a));
        ctx.fill(x, winY + headerH - 2, x + winW, winY + headerH, withAlpha(RED, a));
        ctx.drawTextWithShadow(textRenderer, Text.literal("Infered"), x + 8, winY + 8, withAlpha(RED, a));
        String author = "by thefinalblow";
        ctx.drawTextWithShadow(textRenderer, Text.literal(author), x + winW - textRenderer.getWidth(author) - 6, winY + 10, withAlpha(TEXT_GRAY, a));

         
        for (int i = 0; i < modules.size(); i++) {
            Module m = modules.get(i);
            int ry = winY + headerH + i * ROW_H;

            boolean hovered = mouseX >= x && mouseX < x + winW && mouseY >= ry && mouseY < ry + ROW_H;
            if (hovered && hoverStart[i] < 0) hoverStart[i] = now;
            if (!hovered) hoverStart[i] = -1;
            float hp = hoverStart[i] < 0 ? 0f : easeOut(progress(now, hoverStart[i], HOVER_MS));

             
            int base = (i % 2 == 0) ? ROW_BG : ROW_BG_ALT;
            ctx.fill(x, ry, x + winW, ry + ROW_H, withAlpha(base, a));
            if (hp > 0) {
                ctx.fill(x, ry, x + winW, ry + ROW_H, withAlpha(0x30E02020, (int) (a * hp)));
            }
            if (i == selectedIndex) {
                ctx.fill(x, ry, x + 2, ry + ROW_H, withAlpha(RED, a));
            }

             
            int nameColor = m.isEnabled() || hovered ? TEXT_WHITE : TEXT_GRAY;
            ctx.drawTextWithShadow(textRenderer, Text.literal(m.getName()), x + 10, ry + 7, withAlpha(nameColor, a));

             
            renderSwitch(ctx, x + winW - 26, ry + 7, m.isEnabled(), a, hp);
        }

         
        hollowRect(ctx, x - 1, winY - 1, winW + 2, winH + 2, withAlpha(RED_DIM, a));

         
        String hint = "[LShift] Close  [Enter] Toggle  [R-Click] Settings";
        ctx.drawTextWithShadow(textRenderer, Text.literal(hint), x, winY + winH + 6, withAlpha(0xFF6E6E6E, a));
    }

    private void renderSwitch(DrawContext ctx, int x, int y, boolean on, int a, float hoverP) {
         
        int trackColor = on ? RED : 0xFF3A1010;
        ctx.fill(x, y, x + 16, y + 8, withAlpha(trackColor, a));
        hollowRect(ctx, x - 1, y - 1, 18, 10, withAlpha(RED_DIM, a));
         
        int knobX = on ? x + 9 : x + 1;
        ctx.fill(knobX, y, knobX + 6, y + 8, withAlpha(0xFFF5F5F5, a));
        if (hoverP > 0) {
            ctx.fill(x, y, x + 16, y + 8, withAlpha(0xFFFFFF, (int) (60 * hoverP)));
        }
    }

    private void hollowRect(DrawContext ctx, int x, int y, int w, int h, int color) {
        ctx.fill(x, y, x + w, y + 1, color);
        ctx.fill(x, y + h - 1, x + w, y + h, color);
        ctx.fill(x, y, x + 1, y + h, color);
        ctx.fill(x + w - 1, y, x + w, y + h, color);
    }

    private static int withAlpha(int rgb, int alpha) {
        return (alpha << 24) | (rgb & 0xFFFFFF);
    }
}
