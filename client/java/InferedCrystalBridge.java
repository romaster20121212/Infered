 


















public final class InferedCrystalBridge {
     
    public static volatile boolean ENABLED;

    private InferedCrystalBridge() {}

    public static void beforeKeybinds(Object minecraft) {
        if (ENABLED) {
            tick();
        }
    }

    private static native void tick();
}
