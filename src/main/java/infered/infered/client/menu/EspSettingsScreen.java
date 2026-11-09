package infered.infered.client.menu;

import infered.infered.client.InferedClientHolder;
import infered.infered.client.modules.ChunkEsp;
import infered.infered.client.modules.EntityEsp;
import infered.infered.client.modules.Module;
import infered.infered.client.modules.StashFinder;
import net.minecraft.client.MinecraftClient;
import net.minecraft.client.gui.DrawContext;
import net.minecraft.client.gui.screen.Screen;
import net.minecraft.client.sound.PositionedSoundInstance;
import net.minecraft.sound.SoundEvents;
import net.minecraft.text.Text;
import net.minecraft.util.Util;
import org.lwjgl.glfw.GLFW;

 





public class EspSettingsScreen extends Screen {
    private static final int RED = 0xFFE02020;
    private static final int RED_DIM = 0xFF7A1212;
    private static final int RED_DARK = 0xFF4A0A0A;
    private static final int PANEL = 0xF0101010;
    private static final int ROW_BG = 0xB4151515;
    private static final int ROW_HOVER = 0xD41A1A1A;
    private static final int TINT = 0x60050505;
    private static final int TEXT_WHITE = 0xFFF0F0F0;
    private static final int TEXT_GRAY = 0xFF808080;
    private static final int TEXT_RED = 0xFFE02020;

    private static final long OPEN_MS = 160;
    private static final int ROW_H = 24;
    private static final int BTN_H = 22;

     
    private static final int[] PALETTE = {
        0xFFFFFFFF, 0xFFE02020, 0xFF19FF19, 0xFF1919FF, 0xFFFFFF19,
        0xFFFF8C19, 0xFFB419FF, 0xFFFF199D, 0xFF19FFF2, 0xFFAFAFAF
    };

    private final Module module;
    private final Screen parent;
    private long animStart;
    private int panelW = 220, panelH;

    private record SettingRow(String label, java.util.function.BooleanSupplier getter,
                              java.util.function.Consumer<Boolean> setter) {}

    private final SettingRow[] rows;
    private int selectedRow = -1;

    private EntityEsp entityEsp;    
    private StashFinder stash;      
    private boolean isEsp;          

    public EspSettingsScreen(Module module) {
        this(module, null);
    }

    public EspSettingsScreen(Module module, Screen parent) {
        super(Text.literal(module.getName() + " Settings"));
        this.module = module;
        this.parent = parent;

        if (module instanceof ChunkEsp) {
            isEsp = true;
            rows = new SettingRow[]{
                new SettingRow("Fill",
                    ((ChunkEsp) module)::isFillOn,
                    ((ChunkEsp) module)::setFill)
            };
        } else if (module instanceof EntityEsp esp) {
            entityEsp = esp;
            rows = new SettingRow[]{
                new SettingRow("Fill",
                    esp::isFillOn,
                    esp::setFill)
            };
        } else if (module instanceof StashFinder sf) {
            stash = sf;
            rows = new SettingRow[]{
                new SettingRow("Notifications",
                    sf::isNotificationsOn,
                    sf::setNotificationsOn)
            };
        } else {
            rows = new SettingRow[0];
        }
    }

    @Override
    protected void init() {
        animStart = Util.getMeasuringTimeMs();
        panelH = 30 + rows.length * ROW_H + BTN_H + 26;
        if (entityEsp != null) panelH += EntityEsp.Category.values().length * ROW_H;
        if (stash != null) panelH += 24;
    }

    @Override
    public void close() {
        MinecraftClient.getInstance().setScreen(parent);
    }

    @Override
    public boolean shouldPause() {
        return false;
    }

    private int panelX() {
        return (width - panelW) / 2;
    }

    private int panelY() {
        return (height - panelH) / 2;
    }

    private float slideP() {
        long now = Util.getMeasuringTimeMs();
        float t = Math.min(1f, (now - animStart) / (float) OPEN_MS);
        return 1f - (1f - t) * (1f - t);
    }

    @Override
    public void render(DrawContext ctx, int mouseX, int mouseY, float delta) {
        float p = slideP();
        int a = (int) (255 * p);
        int px = panelX();
        int py = panelY() - (int) ((1f - p) * 14);

        ctx.fill(0, 0, width, height, withAlpha(TINT, a));
        ctx.fill(px, py, px + panelW, py + panelH, withAlpha(PANEL, a));
        hollow(ctx, px - 1, py - 1, panelW + 2, panelH + 2, withAlpha(RED_DIM, a));

         
        ctx.fill(px, py, px + panelW, py + 24, withAlpha(0xFF080808, a));
        ctx.fill(px, py, px + panelW, py + 2, withAlpha(RED, a));  
        ctx.fill(px, py + 22, px + panelW, py + 24, withAlpha(RED, a));  
        ctx.drawTextWithShadow(textRenderer, Text.literal(module.getName() + " Settings"),
            px + 8, py + 8, withAlpha(TEXT_RED, a));

         
        int ry = py + 30;
        for (SettingRow row : rows) {
            boolean on = row.getter().getAsBoolean();
            boolean hovered = mouseX >= px && mouseX < px + panelW && mouseY >= ry && mouseY < ry + ROW_H;

            ctx.fill(px + 2, ry, px + panelW - 2, ry + ROW_H, withAlpha(hovered ? ROW_HOVER : ROW_BG, a));
            ctx.drawTextWithShadow(textRenderer, Text.literal(row.label),
                px + 10, ry + 8, withAlpha(on ? TEXT_RED : TEXT_GRAY, a));
            drawSwitch(ctx, px + panelW - 26, ry + 8, on, a);
            ry += ROW_H;
        }

         
        if (entityEsp != null) {
            for (EntityEsp.Category c : EntityEsp.Category.values()) {
                boolean on = entityEsp.isCategoryOn(c);
                boolean hovered = mouseX >= px && mouseX < px + panelW && mouseY >= ry && mouseY < ry + ROW_H;

                ctx.fill(px + 2, ry, px + panelW - 2, ry + ROW_H, withAlpha(hovered ? ROW_HOVER : ROW_BG, a));
                ctx.drawTextWithShadow(textRenderer, Text.literal(c.label),
                    px + 10, ry + 8, withAlpha(on ? TEXT_RED : TEXT_GRAY, a));
                drawSwitch(ctx, px + panelW - 62, ry + 8, on, a);

                 
                int sx = px + panelW - 30;
                int color = entityEsp.getCategoryColor(c);
                ctx.fill(sx, ry + 5, sx + 14, ry + 19, color);
                hollow(ctx, sx - 1, ry + 4, 16, 16, withAlpha(RED_DIM, a));
                ry += ROW_H;
            }
        }

         
        if (stash != null) {
            int my2 = ry + 2;
            boolean hovL = mouseX >= px + 8 && mouseX < px + 24 && mouseY >= my2 && mouseY < my2 + BTN_H;
            boolean hovR = mouseX >= px + panelW - 24 && mouseX < px + panelW - 8 && mouseY >= my2 && mouseY < my2 + BTN_H;
            ctx.fill(px + 8, my2, px + 24, my2 + BTN_H, withAlpha(hovL ? RED : RED_DARK, a));
            ctx.fill(px + panelW - 24, my2, px + panelW - 8, my2 + BTN_H, withAlpha(hovR ? RED : RED_DARK, a));
            ctx.drawTextWithShadow(textRenderer, Text.literal("-"), px + 14, my2 + 7, withAlpha(TEXT_WHITE, a));
            ctx.drawTextWithShadow(textRenderer, Text.literal("+"), px + panelW - 18, my2 + 7, withAlpha(TEXT_WHITE, a));
            String label = "Min storages: " + stash.getMinStorageCount();
            int tw = textRenderer.getWidth(label);
            ctx.drawTextWithShadow(textRenderer, Text.literal(label),
                px + (panelW - tw) / 2, my2 + 8, withAlpha(TEXT_GRAY, a));
            ry = my2 + BTN_H;
        }

         
        if (isEsp || stash != null) {
            int by = ry + 6;
            boolean hovered = mouseX >= px + 8 && mouseX < px + panelW - 8
                && mouseY >= by && mouseY < by + BTN_H;
            ctx.fill(px + 8, by, px + panelW - 8, by + BTN_H,
                withAlpha(hovered ? RED : RED_DARK, a));
            hollow(ctx, px + 8, by, panelW - 16, BTN_H, withAlpha(RED_DIM, a));
            String label = isEsp ? "Select Blocks" : "Clear Stashes";
            int tw = textRenderer.getWidth(label);
            ctx.drawTextWithShadow(textRenderer, Text.literal(label),
                px + (panelW - tw) / 2, by + 7, withAlpha(hovered ? TEXT_WHITE : TEXT_RED, a));
        }

         
        String hint = entityEsp != null ? "[Esc] Back  ·  click swatch to cycle color" : "[Esc] Back";
        ctx.drawTextWithShadow(textRenderer, Text.literal(hint),
            px, py + panelH + 6, withAlpha(0xFF6E6E6E, a));
    }

    @Override
    public boolean mouseClicked(double mx, double my, int button) {
        if (button != 0) return super.mouseClicked(mx, my, button);
        float p = slideP();
        if (p < 1f) return super.mouseClicked(mx, my, button);
        click(MinecraftClient.getInstance(), (int) mx, (int) my, panelX(), (height - panelH) / 2);
        return super.mouseClicked(mx, my, button);
    }

    private void click(MinecraftClient client, int mx, int my, int px, int py) {
        int ry = py + 30;

         
        for (SettingRow row : rows) {
            if (mx >= px && mx < px + panelW && my >= ry && my < ry + ROW_H) {
                boolean on = row.getter().getAsBoolean();
                row.setter().accept(!on);
                client.getSoundManager().play(PositionedSoundInstance.master(
                    SoundEvents.UI_BUTTON_CLICK, !on ? 1.1f : 0.8f));
                return;
            }
            ry += ROW_H;
        }

         
        if (entityEsp != null) {
            for (EntityEsp.Category c : EntityEsp.Category.values()) {
                if (mx >= px && mx < px + panelW && my >= ry && my < ry + ROW_H) {
                    int sx = px + panelW - 30;
                    if (mx >= sx - 2 && mx < sx + 16) {
                         
                        int cur = entityEsp.getCategoryColor(c);
                        int idx = 0;
                        for (int i = 0; i < PALETTE.length; i++) {
                            if (PALETTE[i] == cur) { idx = i; break; }
                        }
                        entityEsp.setCategoryColor(c, PALETTE[(idx + 1) % PALETTE.length]);
                        client.getSoundManager().play(PositionedSoundInstance.master(
                            SoundEvents.UI_BUTTON_CLICK, 1.2f));
                    } else {
                        entityEsp.setCategoryOn(c, !entityEsp.isCategoryOn(c));
                        client.getSoundManager().play(PositionedSoundInstance.master(
                            SoundEvents.UI_BUTTON_CLICK, 1.0f));
                    }
                    return;
                }
                ry += ROW_H;
            }
        }

         
        if (stash != null) {
            int my2 = ry + 2;
            if (mx >= px + 8 && mx < px + 24 && my >= my2 && my < my2 + BTN_H) {
                stash.setMinStorageCount(stash.getMinStorageCount() - 1);
                client.getSoundManager().play(PositionedSoundInstance.master(
                    SoundEvents.UI_BUTTON_CLICK, 0.9f));
                return;
            }
            if (mx >= px + panelW - 24 && mx < px + panelW - 8 && my >= my2 && my < my2 + BTN_H) {
                stash.setMinStorageCount(stash.getMinStorageCount() + 1);
                client.getSoundManager().play(PositionedSoundInstance.master(
                    SoundEvents.UI_BUTTON_CLICK, 1.1f));
                return;
            }
            ry = my2 + BTN_H;

            int by = ry + 6;
            if (mx >= px + 8 && mx < px + panelW - 8 && my >= by && my < by + BTN_H) {
                stash.clearStashes();
                client.getSoundManager().play(PositionedSoundInstance.master(
                    SoundEvents.UI_BUTTON_CLICK, 0.8f));
            }
            return;
        }

         
        if (isEsp) {
            int by = ry + 6;
            if (mx >= px + 8 && mx < px + panelW - 8 && my >= by && my < by + BTN_H) {
                client.getSoundManager().play(PositionedSoundInstance.master(
                    SoundEvents.UI_BUTTON_CLICK, 1.0f));
                client.setScreen(new BlockSelectorScreen((ChunkEsp) module, this));
            }
        }
    }

    @Override
    public boolean keyPressed(int keyCode, int scanCode, int modifiers) {
        if (keyCode == GLFW.GLFW_KEY_ESCAPE || keyCode == GLFW.GLFW_KEY_BACKSPACE) {
            close();
            return true;
        }
        return super.keyPressed(keyCode, scanCode, modifiers);
    }

    private void drawSwitch(DrawContext ctx, int x, int y, boolean on, int a) {
        ctx.fill(x, y, x + 16, y + 8, withAlpha(on ? RED : RED_DARK, a));
        hollow(ctx, x - 1, y - 1, 18, 10, withAlpha(RED_DIM, a));
        int knobX = on ? x + 9 : x + 1;
        ctx.fill(knobX, y, knobX + 6, y + 8, withAlpha(TEXT_WHITE, a));
    }

    private void hollow(DrawContext ctx, int x, int y, int w, int h, int color) {
        ctx.fill(x, y, x + w, y + 1, color);
        ctx.fill(x, y + h - 1, x + w, y + h, color);
        ctx.fill(x, y, x + 1, y + h, color);
        ctx.fill(x + w - 1, y, x + w, y + h, color);
    }

    private static int withAlpha(int rgb, int alpha) {
        return (alpha << 24) | (rgb & 0xFFFFFF);
    }
}
