import java.lang.reflect.Constructor;
import java.lang.reflect.Method;

 
























public final class InferedItemScaleBridge {
     
    public static volatile boolean ENABLED;

     
    public static volatile float MAIN_SCALE = 1.0F;
    public static volatile float MAIN_X = 0.0F;
    public static volatile float MAIN_Y = 0.0F;
    public static volatile float MAIN_Z = 0.0F;
    public static volatile float MAIN_PITCH = 0.0F;
    public static volatile float MAIN_YAW = 0.0F;
    public static volatile float MAIN_ROLL = 0.0F;

     
    public static volatile float OFF_SCALE = 1.0F;
    public static volatile float OFF_X = 0.0F;
    public static volatile float OFF_Y = 0.0F;
    public static volatile float OFF_Z = 0.0F;
    public static volatile float OFF_PITCH = 0.0F;
    public static volatile float OFF_YAW = 0.0F;
    public static volatile float OFF_ROLL = 0.0F;

     
    private static final double PIVOT_X = 0.56;
    private static final double PIVOT_Y = -0.52;
    private static final double PIVOT_Z = -0.72;

     
    private static Class<?> cachedOwner;
    private static Method cachedTranslate;
    private static Method cachedScale;
    private static Method cachedMultiply;
    private static Method cachedRotationXYZ;
    private static Method cachedConjugate;
    private static Constructor<?> cachedQuaternion;

     
    private static boolean reported;
    private static boolean rotationReported;

     




    private static Object transformedOn;
    private static double transformedSide;
     
    private static float[] transformedView;
     
    private static float[] transformedAngles;

    private InferedItemScaleBridge() {}

     







    public static void apply(Object hand, Object matrices) {
         
         
         
         
        if (InferedHandOutlineBridge.replaying || InferedSwingEchoBridge.replaying) {
            return;
        }
        if (matrices == null) {
            return;
        }
         
         
         
        restoreTransformed(matrices);
        if (!ENABLED) {
            return;
        }
        boolean offHand = isOffHand(hand);
        float[] view = view(offHand);
        if (!(view[0] > 0.0F)) {
            view[0] = 1.0F;
        }
        boolean scaled = view[0] != 1.0F;
        boolean moved = view[1] != 0.0F || view[2] != 0.0F || view[3] != 0.0F;
        boolean turned = view[4] != 0.0F || view[5] != 0.0F || view[6] != 0.0F;
        if (!scaled && !moved && !turned) {
            return;
        }
        resolve(matrices.getClass());
        if (cachedTranslate == null || cachedScale == null) {
            return;
        }
        float[] angles = null;
        if (turned) {
            if (rotatable()) {
                angles = radians(view);
            } else {
                reportRotation(matrices.getClass());
            }
        }
         
        double side = offHand ? -1.0 : 1.0;
        try {
            transform(matrices, side, view, angles, false);
        } catch (Throwable failure) {
             
             
             
            disable(failure);
            return;
        }
        transformedOn = matrices;
        transformedSide = side;
        transformedView = view;
        transformedAngles = angles;
    }

     







    private static void transform(Object matrices, double side, float[] view, float[] angles,
                                  boolean inverse) throws ReflectiveOperationException {
        double px = side * PIVOT_X;
        boolean scaled = view[0] != 1.0F;
        boolean moved = view[1] != 0.0F || view[2] != 0.0F || view[3] != 0.0F;
         
        if (!inverse) {
            translate(matrices, px, PIVOT_Y, PIVOT_Z);
            if (moved) {
                translate(matrices, view[1], view[2], view[3]);
            }
            if (angles != null) {
                multiply(matrices, rotation(angles, false));
            }
            if (scaled) {
                scale(matrices, view[0], view[0], view[0]);
            }
            translate(matrices, -px, -PIVOT_Y, -PIVOT_Z);
        } else {
            float inverseScale = 1.0F / view[0];
            translate(matrices, px, PIVOT_Y, PIVOT_Z);
            if (scaled) {
                scale(matrices, inverseScale, inverseScale, inverseScale);
            }
            if (angles != null) {
                multiply(matrices, rotation(angles, true));
            }
            if (moved) {
                translate(matrices, -view[1], -view[2], -view[3]);
            }
            translate(matrices, -px, -PIVOT_Y, -PIVOT_Z);
        }
    }

     




    private static void restoreTransformed(Object matrices) {
        Object owner = transformedOn;
        if (owner == null) {
            return;
        }
        float[] view = transformedView;
        float[] angles = transformedAngles;
        double side = transformedSide;
        transformedOn = null;
        transformedView = null;
        transformedAngles = null;
        if (owner != matrices) {
             
            return;
        }
        resolve(matrices.getClass());
        if (cachedTranslate == null || cachedScale == null) {
            return;
        }
        try {
            transform(matrices, side, view, angles, true);
        } catch (Throwable failure) {
            disable(failure);
        }
    }

     







    private static boolean isOffHand(Object hand) {
        return hand instanceof Enum && ((Enum<?>) hand).name().contains("OFF_HAND");
    }

     
    private static float[] view(boolean offHand) {
        if (offHand) {
            return new float[] {OFF_SCALE, OFF_X, OFF_Y, OFF_Z, OFF_PITCH, OFF_YAW, OFF_ROLL};
        }
        return new float[] {MAIN_SCALE, MAIN_X, MAIN_Y, MAIN_Z, MAIN_PITCH, MAIN_YAW, MAIN_ROLL};
    }

     
    private static float[] radians(float[] view) {
        return new float[] {
            (float) Math.toRadians(view[4]),
            (float) Math.toRadians(view[5]),
            (float) Math.toRadians(view[6])
        };
    }

     




    private static Object rotation(float[] angles, boolean inverse) throws ReflectiveOperationException {
        Object quaternion = cachedQuaternion.newInstance();
        cachedRotationXYZ.invoke(quaternion, angles[0], angles[1], angles[2]);
        if (inverse) {
            cachedConjugate.invoke(quaternion);
        }
        return quaternion;
    }

    private static boolean rotatable() {
        return cachedMultiply != null && cachedRotationXYZ != null
                && cachedConjugate != null && cachedQuaternion != null;
    }

    private static void translate(Object matrices, double x, double y, double z)
            throws ReflectiveOperationException {
        cachedTranslate.invoke(matrices, x, y, z);
    }

    private static void scale(Object matrices, float x, float y, float z)
            throws ReflectiveOperationException {
        cachedScale.invoke(matrices, x, y, z);
    }

    private static void multiply(Object matrices, Object quaternion)
            throws ReflectiveOperationException {
        cachedMultiply.invoke(matrices, quaternion);
    }

    private static void resolve(Class<?> owner) {
        if (cachedOwner == owner) {
            return;
        }
        Method translate = lookup(owner, "method_22904", double.class, double.class, double.class);
        if (translate == null) {
            translate = lookup(owner, "translate", double.class, double.class, double.class);
        }
        Method scale = lookup(owner, "method_22905", float.class, float.class, float.class);
        if (scale == null) {
            scale = lookup(owner, "scale", float.class, float.class, float.class);
        }
        cachedOwner = owner;
        cachedTranslate = translate;
        cachedScale = scale;
        cachedMultiply = null;
        cachedRotationXYZ = null;
        cachedConjugate = null;
        cachedQuaternion = null;
        if (translate == null || scale == null) {
            report("no translate/scale methods on " + owner.getName());
            return;
        }
        Class<?> quaternion = quaternionClass(owner.getClassLoader());
        if (quaternion == null) {
            return;
        }
        Method multiply = lookup(owner, "method_22907", quaternion);
        if (multiply == null) {
            multiply = lookup(owner, "multiply", quaternion);
        }
        cachedMultiply = multiply;
        cachedRotationXYZ = lookup(quaternion, "rotationXYZ", float.class, float.class, float.class);
        cachedConjugate = lookup(quaternion, "conjugate");
        try {
            cachedQuaternion = quaternion.getConstructor();
        } catch (NoSuchMethodException | SecurityException missing) {
            cachedQuaternion = null;
        }
    }

    private static Class<?> quaternionClass(ClassLoader loader) {
        try {
            return Class.forName("org.joml.Quaternionf", false, loader);
        } catch (ClassNotFoundException | LinkageError missing) {
            return null;
        }
    }

    private static Method lookup(Class<?> owner, String name, Class<?>... parameters) {
        try {
            return owner.getMethod(name, parameters);
        } catch (NoSuchMethodException | SecurityException missing) {
            return null;
        }
    }

    private static void reportRotation(Class<?> owner) {
        if (!rotationReported) {
            rotationReported = true;
            System.err.println("[InferedItemScale] rotation skipped: no org.joml.Quaternionf "
                    + "or multiply on " + owner.getName());
        }
    }

    private static void disable(Throwable failure) {
        ENABLED = false;
        report(failure.toString());
    }

    private static void report(String message) {
        if (!reported) {
            reported = true;
            System.err.println("[InferedItemScale] disabled: " + message);
        }
    }
}
