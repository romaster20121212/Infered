package infered.infered.client.menu;

import infered.infered.client.modules.TestModule676767;
import net.minecraft.client.MinecraftClient;
import net.minecraft.client.gui.DrawContext;
import net.minecraft.client.gui.screen.Screen;
import net.minecraft.client.sound.PositionedSoundInstance;
import net.minecraft.sound.SoundEvents;
import net.minecraft.text.Text;
import net.minecraft.util.Util;
import org.lwjgl.glfw.GLFW;

 
public final class TestModule676767SettingsScreen extends Screen {
    private static final int RED = 0xFFE02020;
    private static final int RED_DIM = 0xFF7A1212;
    private static final int RED_DARK = 0xFF4A0A0A;
    private static final int PANEL = 0xF0101010;
    private static final int ROW_BG = 0xB4151515;
    private static final int ROW_HOVER = 0xD41A1A1A;
    private static final int TEXT_WHITE = 0xFFF0F0F0;
    private static final int TEXT_GRAY = 0xFF9A9A9A;
    private static final int ROW_HEIGHT = 25;
    private static final int PANEL_WIDTH = 310;
    private static final long OPEN_MS = 160;

    private final TestModule676767 module;
    private final Screen parent;
    private long animStart;
    private final int panelHeight = 30 + ROW_HEIGHT * 10 + 26;

    public TestModule676767SettingsScreen(TestModule676767 module) {
        this(module, null);
    }

    public TestModule676767SettingsScreen(TestModule676767 module, Screen parent) {
        super(Text.literal(module.getName() + " Settings"));
        this.module = module;
        this.parent = parent;
    }

    @Override
    protected void init() {
        animStart = Util.getMeasuringTimeMs();
    }

    @Override
    public void close() {
        MinecraftClient.getInstance().setScreen(parent);
    }

    @Override
    public boolean shouldPause() {
        return false;
    }

    @Override
    public void render(DrawContext ctx, int mouseX, int mouseY, float delta) {
        float p = slideProgress();
        int alpha = (int) (255 * p);
        int x = (width - PANEL_WIDTH) / 2;
        int y = (height - panelHeight) / 2 - (int) ((1f - p) * 14);

        ctx.fill(0, 0, width, height, withAlpha(0x60050505, alpha));
        ctx.fill(x, y, x + PANEL_WIDTH, y + panelHeight, withAlpha(PANEL, alpha));
        border(ctx, x - 1, y - 1, PANEL_WIDTH + 2, panelHeight + 2, withAlpha(RED_DIM, alpha));
        ctx.fill(x, y, x + PANEL_WIDTH, y + 24, withAlpha(0xFF080808, alpha));
        ctx.fill(x, y, x + PANEL_WIDTH, y + 2, withAlpha(RED, alpha));
        ctx.fill(x, y + 22, x + PANEL_WIDTH, y + 24, withAlpha(RED, alpha));
        ctx.drawTextWithShadow(textRenderer, Text.literal("Test Module 67 67 67 Settings"),
            x + 9, y + 8, withAlpha(RED, alpha));

        String[] labels = {
            "Wait before obsidian switch",
            "Wait before obsidian place",
            "Wait before verification",
            "Placement confirmation timeout",
            "Wait after crystal switch",
            "Wait before crystal place",
            "Cooldown after sequence",
            "Verify obsidian placement",
            "Restore slot if sequence fails",
            "Skip existing crystal bases"
        };
        int rowY = y + 30;
        for (int i = 0; i < labels.length; i++) {
            boolean hovered = mouseX >= x && mouseX < x + PANEL_WIDTH
                && mouseY >= rowY && mouseY < rowY + ROW_HEIGHT;
            ctx.fill(x + 2, rowY, x + PANEL_WIDTH - 2, rowY + ROW_HEIGHT,
                withAlpha(hovered ? ROW_HOVER : ROW_BG, alpha));
            ctx.drawTextWithShadow(textRenderer, Text.literal(labels[i]),
                x + 9, rowY + 9, withAlpha(TEXT_WHITE, alpha));

            if (i < 7) {
                int minusX = x + PANEL_WIDTH - 82;
                int plusX = x + PANEL_WIDTH - 25;
                drawStepper(ctx, minusX, plusX, rowY + 2, getValue(i), alpha,
                    mouseX, mouseY);
            } else {
                drawSwitch(ctx, x + PANEL_WIDTH - 25, rowY + 8, getToggle(i), alpha);
            }
            rowY += ROW_HEIGHT;
        }

        ctx.drawTextWithShadow(textRenderer, Text.literal("Click −/+ to change ticks  ·  [Esc] Back"),
            x + 8, y + panelHeight + 6, withAlpha(TEXT_GRAY, alpha));
    }

    @Override
    public boolean mouseClicked(double mouseX, double mouseY, int button) {
        if (button != 0 && button != 1) return super.mouseClicked(mouseX, mouseY, button);
        if (slideProgress() < 1f) return super.mouseClicked(mouseX, mouseY, button);

        int x = (width - PANEL_WIDTH) / 2;
        int y = (height - panelHeight) / 2;
        int firstRowY = y + 30;
        for (int i = 0; i < 10; i++) {
            int rowY = firstRowY + i * ROW_HEIGHT;
            if (mouseX < x || mouseX >= x + PANEL_WIDTH || mouseY < rowY || mouseY >= rowY + ROW_HEIGHT) {
                continue;
            }
            if (i < 7) {
                int minusX = x + PANEL_WIDTH - 82;
                int plusX = x + PANEL_WIDTH - 25;
                int delta = button == 1 ? -1 : 1;
                if (mouseX >= minusX && mouseX < minusX + 22) delta = -1;
                else if (mouseX >= plusX && mouseX < plusX + 22) delta = 1;
                else return true;
                changeValue(i, delta);
            } else {
                setToggle(i, !getToggle(i));
            }
            MinecraftClient.getInstance().getSoundManager().play(PositionedSoundInstance.master(
                SoundEvents.UI_BUTTON_CLICK, button == 1 ? 0.85f : 1.0f));
            return true;
        }
        return super.mouseClicked(mouseX, mouseY, button);
    }

    @Override
    public boolean keyPressed(int keyCode, int scanCode, int modifiers) {
        if (keyCode == GLFW.GLFW_KEY_ESCAPE || keyCode == GLFW.GLFW_KEY_BACKSPACE) {
            close();
            return true;
        }
        return super.keyPressed(keyCode, scanCode, modifiers);
    }

    private int getValue(int index) {
        return switch (index) {
            case 0 -> module.getSwitchToObsidianDelay();
            case 1 -> module.getObsidianPlaceDelay();
            case 2 -> module.getVerificationDelay();
            case 3 -> module.getConfirmationTimeout();
            case 4 -> module.getSwitchToCrystalDelay();
            case 5 -> module.getCrystalPlaceDelay();
            case 6 -> module.getCooldownTicks();
            default -> 0;
        };
    }

    private void changeValue(int index, int delta) {
        switch (index) {
            case 0 -> module.setSwitchToObsidianDelay(module.getSwitchToObsidianDelay() + delta);
            case 1 -> module.setObsidianPlaceDelay(module.getObsidianPlaceDelay() + delta);
            case 2 -> module.setVerificationDelay(module.getVerificationDelay() + delta);
            case 3 -> module.setConfirmationTimeout(module.getConfirmationTimeout() + delta);
            case 4 -> module.setSwitchToCrystalDelay(module.getSwitchToCrystalDelay() + delta);
            case 5 -> module.setCrystalPlaceDelay(module.getCrystalPlaceDelay() + delta);
            case 6 -> module.setCooldownTicks(module.getCooldownTicks() + delta);
        }
    }

    private boolean getToggle(int index) {
        return switch (index) {
            case 7 -> module.isVerifyPlacement();
            case 8 -> module.isRestoreSlotOnFailure();
            case 9 -> module.isSkipExistingCrystalBases();
            default -> false;
        };
    }

    private void setToggle(int index, boolean value) {
        switch (index) {
            case 7 -> module.setVerifyPlacement(value);
            case 8 -> module.setRestoreSlotOnFailure(value);
            case 9 -> module.setSkipExistingCrystalBases(value);
        }
    }

    private void drawStepper(DrawContext ctx, int minusX, int plusX, int y, int value,
                             int alpha, int mouseX, int mouseY) {
        int h = 20;
        boolean minusHover = mouseX >= minusX && mouseX < minusX + 22 && mouseY >= y && mouseY < y + h;
        boolean plusHover = mouseX >= plusX && mouseX < plusX + 22 && mouseY >= y && mouseY < y + h;
        ctx.fill(minusX, y, minusX + 22, y + h, withAlpha(minusHover ? RED : RED_DARK, alpha));
        ctx.fill(plusX, y, plusX + 22, y + h, withAlpha(plusHover ? RED : RED_DARK, alpha));
        ctx.drawCenteredTextWithShadow(textRenderer, Text.literal("−"), minusX + 11, y + 6,
            withAlpha(TEXT_WHITE, alpha));
        ctx.drawCenteredTextWithShadow(textRenderer, Text.literal("+"), plusX + 11, y + 6,
            withAlpha(TEXT_WHITE, alpha));
        String valueText = Integer.toString(value);
        ctx.drawCenteredTextWithShadow(textRenderer, Text.literal(valueText),
            (minusX + 22 + plusX) / 2, y + 6, withAlpha(TEXT_WHITE, alpha));
    }

    private void drawSwitch(DrawContext ctx, int x, int y, boolean on, int alpha) {
        ctx.fill(x, y, x + 16, y + 8, withAlpha(on ? RED : RED_DARK, alpha));
        border(ctx, x - 1, y - 1, 18, 10, withAlpha(RED_DIM, alpha));
        ctx.fill(on ? x + 9 : x + 1, y, (on ? x + 9 : x + 1) + 6, y + 8,
            withAlpha(TEXT_WHITE, alpha));
    }

    private float slideProgress() {
        float t = Math.min(1f, (Util.getMeasuringTimeMs() - animStart) / (float) OPEN_MS);
        return 1f - (1f - t) * (1f - t);
    }

    private static void border(DrawContext ctx, int x, int y, int w, int h, int color) {
        ctx.fill(x, y, x + w, y + 1, color);
        ctx.fill(x, y + h - 1, x + w, y + h, color);
        ctx.fill(x, y, x + 1, y + h, color);
        ctx.fill(x + w - 1, y, x + w, y + h, color);
    }

    private static int withAlpha(int color, int alpha) {
        return (alpha << 24) | (color & 0xFFFFFF);
    }
}
