import java.lang.reflect.Method;

 

























public final class InferedHandOutlineBridge {
     
    public static volatile boolean ENABLED;
     
    public static volatile float RED = 0.0F;
    public static volatile float GREEN = 0.0F;
    public static volatile float BLUE = 0.0F;
    public static volatile float ALPHA = 1.0F;
     
    public static volatile float THICKNESS = 0.0015F;
     
    public static volatile int SAMPLES = 8;

     
    private static final int MAX_SAMPLES = 16;
    private static final float MAX_THICKNESS = 0.05F;
     
    private static final double DEPTH = 0.01D;

     
    private static final String BODY_METHOD = "method_3228";
    private static final int BODY_PARAMETERS = 10;
     
    private static final String RENDER_SYSTEM = "com.mojang.blaze3d.systems.RenderSystem";
     
    private static final String FLUSH_METHOD = "method_22993";
     
    private static final String PUSH_METHOD = "method_22903";
    private static final String POP_METHOD = "method_22909";
    private static final String TRANSLATE_METHOD = "method_22904";

     
    static boolean replaying;
     
    private static boolean reported;

     
    private static Class<?> cachedOwner;
    private static Method cachedBody;
    private static Method cachedShaderColor;

    private InferedHandOutlineBridge() {}

     






    public static void apply(
            Object self,
            Object player,
            Object tickDelta,
            Object pitch,
            Object hand,
            Object swing,
            Object item,
            Object equip,
            Object matrices,
            Object vc,
            Object light) {
         
         
         
        if (replaying
                || InferedSwingEchoBridge.replaying
                || self == null
                || matrices == null
                || vc == null) {
            return;
        }
         
         
        if (!ENABLED) {
            return;
        }
        try {
            outline(self, player, tickDelta, pitch, hand, swing, item, equip, matrices, vc, light);
        } catch (ReflectiveOperationException | RuntimeException error) {
            report(error);
        }
    }

     









    static void outlinePose(
            Object self,
            Object player,
            Object tickDelta,
            Object pitch,
            Object hand,
            Object swing,
            Object item,
            Object equip,
            Object matrices,
            Object vc,
            Object light) {
        if (!ENABLED || self == null || matrices == null || vc == null) {
            return;
        }
        try {
            outline(self, player, tickDelta, pitch, hand, swing, item, equip, matrices, vc, light);
        } catch (ReflectiveOperationException | RuntimeException error) {
            report(error);
        }
    }

     
    private static void report(Exception error) {
        if (!reported) {
            reported = true;
            System.err.println("Hand Outline: outline is not drawn: " + error);
        }
    }

    private static void outline(
            Object self,
            Object player,
            Object tickDelta,
            Object pitch,
            Object hand,
            Object swing,
            Object item,
            Object equip,
            Object matrices,
            Object vc,
            Object light)
            throws ReflectiveOperationException {
        Method body = bodyMethod(self.getClass());
        Method shader = shaderColor(self.getClass());
        Class<?> matrixOwner = matrices.getClass();
        Method push = method(matrixOwner, PUSH_METHOD);
        Method pop = method(matrixOwner, POP_METHOD);
        Method translate = method(matrixOwner, TRANSLATE_METHOD, double.class, double.class, double.class);
        Method flush = method(vc.getClass(), FLUSH_METHOD);
        replaying = true;
        try {
            shader.invoke(null, RED, GREEN, BLUE, ALPHA);
            int samples = Math.max(1, Math.min(MAX_SAMPLES, SAMPLES));
            float offset = thickness();
            for (int i = 0; i < samples; i++) {
                double angle = i * 2.0D * Math.PI / samples;
                push.invoke(matrices);
                try {
                    translate.invoke(
                            matrices,
                            Math.cos(angle) * offset,
                            Math.sin(angle) * offset,
                            -DEPTH);
                    body.invoke(
                            self,
                            player,
                            tickDelta,
                            pitch,
                            hand,
                            swing,
                            item,
                            equip,
                            matrices,
                            vc,
                            light);
                } finally {
                    pop.invoke(matrices);
                }
            }
             
            flush.invoke(vc);
        } finally {
            replaying = false;
            shader.invoke(null, 1.0F, 1.0F, 1.0F, 1.0F);
        }
    }

     
    private static float thickness() {
        if (!Float.isFinite(THICKNESS)) {
            return 0.0F;
        }
        return Math.max(0.0F, Math.min(MAX_THICKNESS, THICKNESS));
    }

    private static Method bodyMethod(Class<?> owner) throws NoSuchMethodException {
        if (owner != cachedOwner || cachedBody == null) {
            Method found = null;
            for (Method method : owner.getDeclaredMethods()) {
                if (method.getName().equals(BODY_METHOD)
                        && method.getParameterCount() == BODY_PARAMETERS) {
                    found = method;
                    break;
                }
            }
            if (found == null) {
                throw new NoSuchMethodException(BODY_METHOD + " on " + owner.getName());
            }
            found.setAccessible(true);
            cachedOwner = owner;
            cachedBody = found;
        }
        return cachedBody;
    }

    private static Method shaderColor(Class<?> owner) throws ReflectiveOperationException {
        if (cachedShaderColor == null) {
            Class<?> renderSystem = Class.forName(RENDER_SYSTEM, true, owner.getClassLoader());
            cachedShaderColor =
                    renderSystem.getMethod(
                            "setShaderColor", float.class, float.class, float.class, float.class);
        }
        return cachedShaderColor;
    }

    private static Method method(Class<?> owner, String name, Class<?>... parameters)
            throws NoSuchMethodException {
        Method found = owner.getMethod(name, parameters);
        found.setAccessible(true);
        return found;
    }
}
