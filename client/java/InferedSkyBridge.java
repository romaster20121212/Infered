 










public final class InferedSkyBridge {
     
    public static volatile boolean ENABLED;

     




    public static volatile boolean HIDE_VANILLA;

     







    public static native void drawSky();

     
    public static volatile int RED = 255;

     
    public static volatile int GREEN = 255;

     
    public static volatile int BLUE = 255;

    private InferedSkyBridge() {}

     







    public static int tint(int vanilla, Object level) {
        if (!ENABLED) {
            return vanilla;
        }
        int red = clamp(RED);
        int green = clamp(GREEN);
        int blue = clamp(BLUE);
        return (vanilla & 0xFF000000) | (red << 16) | (green << 8) | blue;
    }

    private static int clamp(int channel) {
        return Math.max(0, Math.min(255, channel));
    }
}
