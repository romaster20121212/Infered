package infered.infered.client.render;

import net.minecraft.client.gui.DrawContext;
import net.minecraft.client.toast.Toast;
import net.minecraft.client.toast.ToastManager;
import net.minecraft.client.sound.PositionedSoundInstance;
import net.minecraft.item.ItemStack;
import net.minecraft.item.Items;
import net.minecraft.sound.SoundEvents;
import net.minecraft.text.Text;
import net.minecraft.util.Identifier;

 




public class InferedToast implements Toast {
    private static final long DURATION_MS = 5000;
    private static final Identifier TEXTURE = Identifier.ofVanilla("toast/advancement");

    private static final int TITLE_RED = 0xFFE02020;    
    private static final int TEXT_GRAY = 0xFFDCDCDC;

    private static final int EDGE = 0xFF3A0E0E;         
    private static final int BODY = 0xF0101010;         
    private static final int ACCENT = 0xFFE02020;

    private final Text title;
    private final Text text;
    private final ItemStack icon;
    private boolean playedSound;
    private long start = -1;
    private Visibility visibility = Visibility.SHOW;

    public InferedToast(String title, String text, ItemStack icon) {
        this.title = Text.literal(title);
        this.text = Text.literal(text);
        this.icon = icon != null ? icon : new ItemStack(Items.CHEST);
    }

    public static void show(String title, String text) {
        var client = net.minecraft.client.MinecraftClient.getInstance();
        client.getToastManager().add(new InferedToast(title, text, null));
    }

    @Override
    public Object getType() {
        return TYPE;
    }

    @Override
    public int getWidth() {
        return 160;
    }

    @Override
    public int getHeight() {
        return 32;
    }

    @Override
    public Visibility getVisibility() {
        return visibility;
    }

    @Override
    public void update(ToastManager manager, long time) {
         
         
        if (start == -1) start = time;
        visibility = time - start >= DURATION_MS ? Visibility.HIDE : Visibility.SHOW;

        if (!playedSound) {
            var client = net.minecraft.client.MinecraftClient.getInstance();
            client.getSoundManager().play(PositionedSoundInstance.master(
                SoundEvents.BLOCK_NOTE_BLOCK_CHIME.value(), 0.7f));
            playedSound = true;
        }
    }

    @Override
    public void draw(DrawContext ctx, net.minecraft.client.font.TextRenderer font, long now) {
        int w = getWidth(), h = getHeight();
        ctx.fill(0, 0, w, h, BODY);
        hollow(ctx, 0, 0, w, h, EDGE);
        ctx.fill(0, 0, w, 1, ACCENT);
        ctx.fill(0, h - 1, w, h, ACCENT);

        ctx.drawItem(icon, 6, 8);
        ctx.drawText(font, title, 28, 7, TITLE_RED, false);
        ctx.drawText(font, text, 28, 18, TEXT_GRAY, false);
    }

    private static void hollow(DrawContext ctx, int x, int y, int w, int h, int color) {
        ctx.fill(x, y, x + w, y + 1, color);
        ctx.fill(x, y + h - 1, x + w, y + h, color);
        ctx.fill(x, y, x + 1, y + h, color);
        ctx.fill(x + w - 1, y, x + w, y + h, color);
    }
}
