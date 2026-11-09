package infered.infered.client;

import net.minecraft.client.gui.DrawContext;

import java.util.HashMap;
import java.util.Map;

 







public final class PixelFont {
    private static final Map<Character, int[]> GLYPHS = new HashMap<>();

    private static void g(char c, int r0, int r1, int r2, int r3, int r4, int r5, int r6) {
        GLYPHS.put(c, new int[]{r0, r1, r2, r3, r4, r5, r6});
    }

    static {
         
        g('A', 14, 17, 17, 31, 17, 17, 17);
        g('B', 30, 17, 17, 30, 17, 17, 30);
        g('C', 14, 17, 16, 16, 16, 17, 14);
        g('D', 30, 17, 17, 17, 17, 17, 30);
        g('E', 31, 16, 16, 30, 16, 16, 31);
        g('F', 31, 16, 16, 30, 16, 16, 16);
        g('G', 14, 17, 16, 23, 17, 17, 14);
        g('H', 17, 17, 17, 31, 17, 17, 17);
        g('I', 14, 4, 4, 4, 4, 4, 14);
        g('J', 1, 1, 1, 1, 17, 17, 14);
        g('K', 17, 18, 20, 24, 20, 18, 17);
        g('L', 16, 16, 16, 16, 16, 16, 31);
        g('M', 17, 27, 21, 17, 17, 17, 17);
        g('N', 17, 25, 21, 19, 17, 17, 17);
        g('O', 14, 17, 17, 17, 17, 17, 14);
        g('P', 30, 17, 17, 30, 16, 16, 16);
        g('Q', 14, 17, 17, 17, 21, 18, 13);
        g('R', 30, 17, 17, 30, 20, 18, 17);
        g('S', 14, 17, 16, 14, 1, 17, 14);
        g('T', 31, 4, 4, 4, 4, 4, 4);
        g('U', 17, 17, 17, 17, 17, 17, 14);
        g('V', 17, 17, 17, 17, 17, 10, 4);
        g('W', 17, 17, 17, 21, 21, 27, 17);
        g('X', 17, 17, 10, 4, 10, 17, 17);
        g('Y', 17, 17, 10, 4, 4, 4, 4);
        g('Z', 31, 1, 2, 4, 8, 16, 31);
         
        g('a', 14, 17, 17, 15, 1, 1, 14);
        g('b', 16, 16, 16, 30, 17, 17, 30);
        g('e', 14, 17, 17, 31, 16, 17, 14);
        g('f', 6, 9, 8, 28, 8, 8, 8);
        g('h', 16, 16, 16, 30, 17, 17, 17);
        g('i', 8, 0, 8, 8, 8, 8, 8);
        g('l', 8, 8, 8, 8, 8, 8, 8);
        g('n', 16, 16, 30, 17, 17, 17, 17);
        g('o', 14, 17, 17, 17, 17, 17, 14);
        g('t', 8, 8, 31, 8, 8, 8, 6);
        g('w', 17, 17, 17, 21, 21, 21, 10);
        g('y', 17, 17, 17, 14, 2, 4, 12);
         
        g('0', 14, 17, 19, 21, 25, 17, 14);
        g('1', 4, 12, 4, 4, 4, 4, 14);
        g('2', 14, 17, 1, 2, 4, 8, 31);
        g('3', 30, 1, 1, 14, 1, 1, 30);
        g('4', 2, 6, 10, 18, 31, 2, 2);
        g('5', 31, 16, 30, 1, 1, 17, 14);
        g('6', 6, 8, 16, 30, 17, 17, 14);
        g('7', 31, 1, 2, 4, 8, 8, 8);
        g('8', 14, 17, 17, 14, 17, 17, 14);
        g('9', 14, 17, 17, 15, 1, 2, 12);
         
        g('%', 17, 18, 4, 8, 9, 17, 0);
        g('.', 0, 0, 0, 0, 0, 0, 4);
        g('-', 0, 0, 0, 31, 0, 0, 0);
        g(':', 0, 4, 0, 0, 0, 4, 0);
        g(' ', 0, 0, 0, 0, 0, 0, 0);
    }

     
    public static int width(String text, int scale) {
        if (text.isEmpty()) return 0;
        return (text.length() * 6 - 1) * scale;
    }

    public static int height(int scale) {
        return 7 * scale;
    }

     
    public static void draw(DrawContext ctx, String text, int x, int y, int color, int scale) {
        int cx = x;
        for (int ci = 0; ci < text.length(); ci++) {
            int[] glyph = GLYPHS.get(text.charAt(ci));
            if (glyph != null) {
                for (int row = 0; row < 7; row++) {
                    int bits = glyph[row];
                    for (int col = 0; col < 5; col++) {
                        if ((bits & (1 << (4 - col))) != 0) {
                            int px = cx + col * scale;
                            int py = y + row * scale;
                            ctx.fill(px, py, px + scale, py + scale, color);
                        }
                    }
                }
            }
            cx += 6 * scale;
        }
    }

    private PixelFont() {}
}
