import java.lang.reflect.Method;

 







public final class InferedTotemPopBridge {
     
    public static volatile float SCALE = 1.0f;

    private static volatile Class<?> cachedOwner;
    private static volatile Method cachedScale;
    private static volatile boolean reported;

     
    public static void scale(Object matrices, float x, float y, float z) {
        float factor = SCALE;
        try {
            resolve(matrices.getClass()).invoke(matrices, x * factor, y * factor, z * factor);
        } catch (ReflectiveOperationException | RuntimeException failure) {
            if (!reported) {
                reported = true;
                System.err.println("[InferedTotemPop] scale skipped: " + failure);
            }
        }
    }

    private static Method resolve(Class<?> owner) throws NoSuchMethodException {
        if (cachedOwner != owner) {
            Method scale;
            try {
                scale = owner.getMethod("method_22905", float.class, float.class, float.class);
            } catch (NoSuchMethodException missing) {
                scale = owner.getMethod("scale", float.class, float.class, float.class);
            }
            cachedScale = scale;
            cachedOwner = owner;
        }
        return cachedScale;
    }

    private InferedTotemPopBridge() {}
}
