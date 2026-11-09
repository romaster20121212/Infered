 















public final class InferedCrystalOptimizerBridge {
     
    public static volatile boolean ENABLED;

    private InferedCrystalOptimizerBridge() {}

    public static void beforeAttack(Object target) {
        if (ENABLED && target != null) {
            noteAttack(target);
        }
    }

    private static native void noteAttack(Object target);
}
