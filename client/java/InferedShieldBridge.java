import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;

 













public final class InferedShieldBridge {
     
    public static volatile boolean ENABLED;

     
    public static volatile boolean MAIN_HAND = true;

     
    public static volatile boolean OFF_HAND = true;

    private static final Object[] NO_ARGS = new Object[0];

     
    private static Class<?> cachedOwner;
    private static Method getItem;
    private static Object shield;
    private static Object empty;
    private static boolean reported;

    private InferedShieldBridge() {}

     







    public static Object hide(Object hand, Object stack) {
        if (!ENABLED || stack == null) {
            return stack;
        }
        if (isOffHand(hand) ? !OFF_HAND : !MAIN_HAND) {
            return stack;
        }
        if (!resolve(stack.getClass())) {
            return stack;
        }
        try {
            return getItem.invoke(stack, NO_ARGS) == shield ? empty : stack;
        } catch (Throwable failure) {
             
             
             
            disable(failure);
            return stack;
        }
    }

     




    private static boolean isOffHand(Object hand) {
        return hand instanceof Enum && ((Enum<?>) hand).name().contains("OFF_HAND");
    }

     





    private static boolean resolve(Class<?> owner) {
        if (cachedOwner == owner) {
            return getItem != null && shield != null && empty != null;
        }
        cachedOwner = owner;
        getItem = method(owner, "method_7909", "getItem");
        empty = staticField(owner, "field_8037", "EMPTY");
        Class<?> items = load(owner, "net.minecraft.class_1802", "net.minecraft.world.item.Items");
        shield = items == null ? null : staticField(items, "field_8255", "SHIELD");
        if (getItem == null || empty == null || shield == null) {
            disable(new IllegalStateException("shield members not found on " + owner.getName()));
            return false;
        }
        return true;
    }

    private static Class<?> load(Class<?> owner, String intermediary, String mojmap) {
        ClassLoader loader = owner.getClassLoader();
        for (String name : new String[] {intermediary, mojmap}) {
            try {
                return Class.forName(name, true, loader);
            } catch (ClassNotFoundException | LinkageError absent) {
                 
            }
        }
        return null;
    }

    private static Method method(Class<?> owner, String intermediary, String mojmap) {
        for (String name : new String[] {intermediary, mojmap}) {
            try {
                return owner.getMethod(name);
            } catch (NoSuchMethodException | SecurityException absent) {
                 
            }
        }
        return null;
    }

    private static Object staticField(Class<?> owner, String intermediary, String mojmap) {
        for (String name : new String[] {intermediary, mojmap}) {
            try {
                Field field = owner.getField(name);
                if (Modifier.isStatic(field.getModifiers())) {
                    return field.get(null);
                }
            } catch (ReflectiveOperationException | RuntimeException absent) {
                 
            }
        }
        return null;
    }

    private static void disable(Throwable failure) {
        ENABLED = false;
        report(failure.toString());
    }

    private static void report(String message) {
        if (!reported) {
            reported = true;
            System.err.println("[InferedShield] disabled: " + message);
        }
    }
}
