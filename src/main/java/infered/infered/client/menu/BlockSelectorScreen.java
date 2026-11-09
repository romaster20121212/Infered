package infered.infered.client.menu;

import infered.infered.client.modules.ChunkEsp;
import net.minecraft.client.MinecraftClient;
import net.minecraft.client.gui.DrawContext;
import net.minecraft.client.gui.screen.Screen;
import net.minecraft.client.gui.widget.TextFieldWidget;
import net.minecraft.client.sound.PositionedSoundInstance;
import net.minecraft.sound.SoundEvents;
import net.minecraft.text.Text;
import net.minecraft.util.Util;
import org.lwjgl.glfw.GLFW;

import java.util.ArrayList;
import java.util.List;

 





public class BlockSelectorScreen extends Screen {
    private static final int RED = 0xFFE02020;
    private static final int RED_DIM = 0xFF7A1212;
    private static final int RED_DARK = 0xFF4A0A0A;
    private static final int PANEL = 0xF0101010;
    private static final int ROW_BG = 0xB4151515;
    private static final int ROW_HOVER = 0xD41A1A1A;
    private static final int TINT = 0x60050505;
    private static final int TEXT_WHITE = 0xFFF0F0F0;
    private static final int TEXT_GRAY = 0xFF808080;
    private static final int SEL_BG = 0xFF5A1616;

    private static final long OPEN_MS = 160;
    private static final int ROW = 10;

    private final ChunkEsp esp;
    private final Screen parent;
    private long animStart;
    private long lastFlash;

    private final int panelW = 440, panelH = 250;
    private final int LEFT_W = 232, RIGHT_W = 140;
    private final int SEARCH_H = 14;

    private TextFieldWidget search;
    private String filter = "";

    private final List<String> availShown = new ArrayList<>();
    private final List<String> enabledShown = new ArrayList<>();
    private int scrollAvail, scrollEnabled;
    private int selAvail = -1;

    public BlockSelectorScreen(ChunkEsp esp) {
        this(esp, null);
    }

    public BlockSelectorScreen(ChunkEsp esp, Screen parent) {
        super(Text.literal(esp.getName() + " Blocks"));
        this.esp = esp;
        this.parent = parent;
    }

    @Override
    protected void init() {
        animStart = Util.getMeasuringTimeMs();
        if (search == null) {
            search = new TextFieldWidget(textRenderer, 0, 0, LEFT_W, SEARCH_H, Text.literal("Search"));
            search.setMaxLength(64);
            search.setChangedListener(s -> {
                filter = s.toLowerCase().trim();
                scrollAvail = 0;
                selAvail = -1;
            });
        }
        search.setX(panelX() + 8);
        search.setY(panelY() + 28);
        search.setWidth(LEFT_W);
        search.setHeight(SEARCH_H);
        addSelectableChild(search);
        setInitialFocus(search);
    }

    private int panelX() {
        return (width - panelW) / 2;
    }

    private int panelY() {
        return (height - panelH) / 2;
    }

    private int listTop() {
        return panelY() + 46;
    }

    private int listBottom() {
        return panelY() + panelH - 24;
    }

    private int visibleRows() {
        return Math.max(1, (listBottom() - listTop()) / ROW);
    }

    private float slideP() {
        long now = Util.getMeasuringTimeMs();
        float t = Math.min(1f, (now - animStart) / (float) OPEN_MS);
        return 1f - (1f - t) * (1f - t);
    }

    private boolean matches(String id) {
        return filter.isEmpty() || id.contains(filter);
    }

    private void rebuildLists() {
        availShown.clear();
        for (String id : esp.availableBlocks()) {
            if (!esp.selectedBlocks.contains(id) && matches(id)) {
                availShown.add(id);
            }
        }
        enabledShown.clear();
        for (String id : esp.selectedBlocks) {
            enabledShown.add(id);
        }
        enabledShown.sort(String::compareTo);
        clampScrolls();
    }

    private void clampScrolls() {
        int vis = visibleRows();
        scrollAvail = Math.max(0, Math.min(scrollAvail, Math.max(0, availShown.size() - vis)));
        scrollEnabled = Math.max(0, Math.min(scrollEnabled, Math.max(0, enabledShown.size() - vis)));
    }

    @Override
    public void tick() {
         
    }

    @Override
    public void render(DrawContext ctx, int mouseX, int mouseY, float delta) {
        rebuildLists();
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
        ctx.drawTextWithShadow(textRenderer, Text.literal(esp.getName() + " Blocks"),
            px + 8, py + 8, withAlpha(RED, a));

         
        int rx = px + panelW - RIGHT_W - 8;
        ctx.drawTextWithShadow(textRenderer, Text.literal("Enabled"),
            rx, py + 30, withAlpha(TEXT_GRAY, a));
        int ry = listTop();
        for (int i = scrollEnabled; i < enabledShown.size() && ry + ROW <= listBottom(); i++) {
            String id = enabledShown.get(i);
            boolean hovered = mouseX >= rx && mouseX < rx + RIGHT_W && mouseY >= ry && mouseY < ry + ROW;
            ctx.fill(rx, ry, rx + RIGHT_W, ry + ROW, withAlpha(hovered ? SEL_BG : ROW_BG, a));
            ctx.drawText(textRenderer, shorten(id, RIGHT_W - 6), rx + 3, ry + 1,
                withAlpha(hovered ? RED : TEXT_WHITE, a), false);
            ry += ROW;
        }
        if (enabledShown.isEmpty()) {
            ctx.drawText(textRenderer, "(none)", rx + 3, listTop() + 1,
                withAlpha(TEXT_GRAY, a), false);
        }

         
        search.render(ctx, mouseX, mouseY, delta);
        int lx = px + 8;
        ry = listTop();
        for (int i = scrollAvail; i < availShown.size() && ry + ROW <= listBottom(); i++) {
            String id = availShown.get(i);
            boolean sel = i == selAvail;
            boolean hovered = mouseX >= lx && mouseX < lx + LEFT_W && mouseY >= ry && mouseY < ry + ROW;
            ctx.fill(lx, ry, lx + LEFT_W, ry + ROW, withAlpha(sel ? SEL_BG : (hovered ? ROW_HOVER : ROW_BG), a));
            ctx.drawText(textRenderer, shorten(id, LEFT_W - 6), lx + 3, ry + 1,
                withAlpha(sel || hovered ? TEXT_WHITE : TEXT_GRAY, a), false);
            ry += ROW;
        }
        if (availShown.isEmpty()) {
            ctx.drawText(textRenderer, filter.isEmpty() ? "(all added)" : "(no match)",
                lx + 3, listTop() + 1, withAlpha(TEXT_GRAY, a), false);
        }

         
        int addX = lx + LEFT_W + 6;
        int addW = rx - addX - 6;
        int addY = (listTop() + listBottom()) / 2 - 8;
        boolean addHover = mouseX >= addX && mouseX < addX + addW
            && mouseY >= addY && mouseY < addY + 16;
        ctx.fill(addX, addY, addX + addW, addY + 16,
            withAlpha(addHover ? RED : RED_DARK, a));
        hollow(ctx, addX, addY, addW, 16, withAlpha(RED_DIM, a));
        String addLabel = "Add";
        ctx.drawTextWithShadow(textRenderer, Text.literal(addLabel),
            addX + (addW - textRenderer.getWidth(addLabel)) / 2, addY + 4,
            withAlpha(addHover ? TEXT_WHITE : RED, a));

         
        String count = availShown.size() + " available, " + enabledShown.size() + " enabled";
        ctx.drawText(textRenderer, count, px + 8, py + panelH - 14,
            withAlpha(TEXT_GRAY, a), false);
        String hint = "[Enter] Add   [Click right row] Remove";
        ctx.drawText(textRenderer, hint, rx, py + panelH - 14,
            withAlpha(TEXT_GRAY, a), false);
    }

    private static String shorten(String id, int maxW) {
        return id;  
    }

    @Override
    public boolean mouseClicked(double mx, double my, int button) {
        if (button == 0 && slideP() >= 1f) {
            int px = panelX();
            int py = (height - panelH) / 2;
            int x = (int) mx, y = (int) my;

             
            int rx = px + panelW - RIGHT_W - 8;
            if (x >= rx && x < rx + RIGHT_W && y >= listTop() && y < listBottom()
                && !enabledShown.isEmpty()) {
                int idx = scrollEnabled + (y - listTop()) / ROW;
                if (idx >= 0 && idx < enabledShown.size()) {
                    esp.removeBlock(enabledShown.get(idx));
                    playClick(0.8f);
                    return true;
                }
            }

             
            int lx = px + 8;
            if (x >= lx && x < lx + LEFT_W && y >= listTop() && y < listBottom()
                && !availShown.isEmpty()) {
                int idx = scrollAvail + (y - listTop()) / ROW;
                if (idx >= 0 && idx < availShown.size()) {
                    selAvail = idx;
                    return true;
                }
            }

             
            int addX = lx + LEFT_W + 6;
            int addW = (px + panelW - RIGHT_W - 8) - addX - 6;
            int addY = (listTop() + listBottom()) / 2 - 8;
            if (x >= addX && x < addX + addW && y >= addY && y < addY + 16) {
                String toAdd = selAvail >= 0 && selAvail < availShown.size()
                    ? availShown.get(selAvail)
                    : (availShown.isEmpty() ? null : availShown.get(0));
                if (toAdd != null) {
                    esp.addBlock(toAdd);
                    selAvail = -1;
                    playClick(1.1f);
                    return true;
                }
            }
        }
        return super.mouseClicked(mx, my, button);
    }

    @Override
    public boolean mouseScrolled(double mx, double my, double horizontal, double vertical) {
        int px = panelX();
        int rx = px + panelW - RIGHT_W - 8;
        int dir = vertical > 0 ? -3 : 3;
        if (mx >= rx) {
            scrollEnabled += dir;
        } else {
            scrollAvail += dir;
        }
        clampScrolls();
        return true;
    }

    @Override
    public boolean keyPressed(int keyCode, int scanCode, int modifiers) {
        if (keyCode == GLFW.GLFW_KEY_ESCAPE) {
            close();
            return true;
        }
        if (keyCode == GLFW.GLFW_KEY_ENTER || keyCode == GLFW.GLFW_KEY_KP_ENTER) {
            if (!availShown.isEmpty()) {
                String toAdd = selAvail >= 0 && selAvail < availShown.size()
                    ? availShown.get(selAvail) : availShown.get(0);
                esp.addBlock(toAdd);
                selAvail = -1;
                playClick(1.1f);
            }
            return true;
        }
        return super.keyPressed(keyCode, scanCode, modifiers);
    }

    @Override
    public void close() {
        MinecraftClient.getInstance().setScreen(parent);
    }

    @Override
    public boolean shouldPause() {
        return false;
    }

    private void playClick(float pitch) {
        lastFlash = Util.getMeasuringTimeMs();
        MinecraftClient.getInstance().getSoundManager().play(
            PositionedSoundInstance.master(SoundEvents.UI_BUTTON_CLICK, pitch));
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
