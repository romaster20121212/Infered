import java.lang.reflect.*;
import java.util.*;

 
public final class InferedItemChamsBridge {
    private static final int MAX_PLAYERS = 40;
    private static final float[] OUT = new float[MAX_PLAYERS * 8192 * 5];
    private static final int[] TRIANGLES = {0, 1, 2, 0, 2, 3};
    private static final float[] QUAD = new float[20];
    private static int size, quadSize;
    private static ClassLoader loader;
    private static Class<?> consumer;
    private static Object sink;
    private static Method getRenderer, updateState, getModel, setAngles, translateToHand, itemRender;
    private static Method push, pop, translate, rotate, scale, transforms, rendererScale, offset;
    private static Constructor<?> stackCtor, quaternionCtor;
    private static Field baseScale, bodyYaw, sleepingDirection, eyeHeight;
    private static Method vecX, vecY, vecZ, directionX, directionZ;
    private static Object rightArm, leftArm, mainHand, offHand, thirdLeft, thirdRight;
    private static Class<?> slotClass;

    private InferedItemChamsBridge() {}

    private static void init(ClassLoader cl) throws Exception {
        if (loader == cl) return;
        Class<?> entity = c("class_1297", cl), dispatcher = c("class_898", cl);
        Class<?> renderer = c("class_897", cl), living = c("class_922", cl);
        Class<?> state = c("class_10017", cl), livingState = c("class_10042", cl);
        Class<?> matrices = c("class_4587", cl), armModel = c("class_3881", cl);
        Class<?> itemRenderer = c("class_918", cl), livingEntity = c("class_1309", cl);
        Class<?> itemStack = c("class_1799", cl), display = c("class_811", cl);
        Class<?> arm = c("class_1306", cl), slot = c("class_1304", cl);
        slotClass = slot;
        Class<?> provider = c("class_4597", cl), world = c("class_1937", cl);
        Class<?> direction = c("class_2350", cl), vec = c("class_243", cl);
        Class<?> quaternion = Class.forName("org.joml.Quaternionf", true, cl);
        consumer = c("class_4588", cl);
        getRenderer = dispatcher.getMethod("method_3953", entity);
        updateState = renderer.getMethod("method_62425", entity, float.class);
        getModel = living.getMethod("method_4038");
        setAngles = c("class_583", cl).getMethod("method_2819", livingState);
        translateToHand = armModel.getMethod("method_2803", arm, matrices);
        stackCtor = matrices.getConstructor(); quaternionCtor = quaternion.getConstructor();
        push = matrices.getMethod("method_22903"); pop = matrices.getMethod("method_22909");
        translate = matrices.getMethod("method_22904", double.class, double.class, double.class);
        rotate = matrices.getMethod("method_22907", quaternion);
        scale = matrices.getMethod("method_22905", float.class, float.class, float.class);
        transforms = living.getDeclaredMethod("method_4058", livingState, matrices, float.class, float.class);
        transforms.setAccessible(true);
        rendererScale = living.getDeclaredMethod("method_4042", livingState, matrices);
        rendererScale.setAccessible(true);
        offset = renderer.getMethod("method_23169", state);
        baseScale = livingState.getField("field_53453"); bodyYaw = livingState.getField("field_53446");
        sleepingDirection = livingState.getField("field_53463"); eyeHeight = state.getField("field_53331");
        vecX = vec.getMethod("method_10216"); vecY = vec.getMethod("method_10214"); vecZ = vec.getMethod("method_10215");
        directionX = direction.getMethod("method_10148"); directionZ = direction.getMethod("method_10165");
        itemRender = itemRenderer.getMethod("method_23177", livingEntity, itemStack, display, boolean.class,
                matrices, provider, world, int.class, int.class, int.class);
        rightArm = arm.getField("field_6182").get(null); leftArm = arm.getField("field_6183").get(null);
        mainHand = slot.getField("field_6173").get(null); offHand = slot.getField("field_6171").get(null);
        thirdLeft = display.getField("field_4323").get(null); thirdRight = display.getField("field_4320").get(null);
        sink = Proxy.newProxyInstance(cl, new Class<?>[]{consumer}, (proxy, method, args) -> {
            String name = method.getName();
            if (method.isDefault() && !name.equals("method_23919"))
                return InvocationHandler.invokeDefault(proxy, method, args);
            if (name.equals("method_23919")) {
                add(((Number)args[0]).floatValue(), ((Number)args[1]).floatValue(), ((Number)args[2]).floatValue(),
                    ((Number)args[4]).floatValue(), ((Number)args[5]).floatValue());
            } else if (name.equals("method_22912")) {
                QUAD[quadSize * 5] = ((Number)args[0]).floatValue();
                QUAD[quadSize * 5 + 1] = ((Number)args[1]).floatValue();
                QUAD[quadSize * 5 + 2] = ((Number)args[2]).floatValue();
            } else if (name.equals("method_22913")) {
                if (quadSize < 4) {
                    QUAD[quadSize * 5 + 3] = ((Number)args[0]).floatValue();
                    QUAD[quadSize * 5 + 4] = ((Number)args[1]).floatValue();
                    quadSize++;
                    if (quadSize == 4) flushQuad();
                }
            } else if (name.equals("toString")) return "InferedItemChamsGeometry";
            else if (name.equals("hashCode")) return System.identityHashCode(proxy);
            else if (name.equals("equals")) return proxy == args[0];
            if (method.getReturnType() == void.class) return null;
            if (method.getReturnType() == consumer) return proxy;
            throw new UnsupportedOperationException(method.toString());
        });
         
        Object buffers = Proxy.newProxyInstance(cl, new Class<?>[]{provider}, (proxy, method, args) -> {
            if (method.getName().equals("getBuffer")) return sink;
            if (method.getName().equals("toString")) return "InferedItemChamsBuffers";
            if (method.getName().equals("hashCode")) return System.identityHashCode(proxy);
            if (method.getName().equals("equals")) return proxy == args[0];
            if (method.isDefault()) return InvocationHandler.invokeDefault(proxy, method, args);
            if (method.getReturnType() == void.class) return null;
            throw new UnsupportedOperationException(method.toString());
        });
        BUFFER_PROVIDER = buffers;
        loader = cl;
    }

    private static Object BUFFER_PROVIDER;
    private static Class<?> c(String name, ClassLoader cl) throws Exception {
        return Class.forName("net.minecraft." + name, true, cl);
    }

    private static Method getTextureManager, getAtlasTexture, getTextureId;
    private static Object blockAtlasLocation;
    private static boolean atlasFailed;

     









    public static synchronized int atlas(Object client) {
        if (atlasFailed) return 0;
        try {
            if (getTextureManager == null) {
                init(client.getClass().getClassLoader());
                getTextureManager = client.getClass().getMethod("method_1531");
                 
                 
                 
                blockAtlasLocation = c("class_1059", loader).getField("field_5275").get(null);
                getAtlasTexture = c("class_1060", loader).getMethod("method_4619",
                    c("class_2960", loader));
                getTextureId = c("class_1044", loader).getMethod("method_4624");
            }
            Object manager = getTextureManager.invoke(client);
            Object texture = getAtlasTexture.invoke(manager, blockAtlasLocation);
            return (Integer) getTextureId.invoke(texture);
        } catch (Throwable error) {
            atlasFailed = true;
            return 0;
        }
    }
    private static void add(float x, float y, float z, float u, float v) {
        if (!Float.isFinite(x + y + z + u + v) || size + 5 > OUT.length)
            throw new IllegalStateException("Invalid Item Chams vertex");
        OUT[size++] = x; OUT[size++] = y; OUT[size++] = z; OUT[size++] = u; OUT[size++] = v;
    }
    private static void flushQuad() {
        for (int i : TRIANGLES) for (int j = 0; j < 5; j++) OUT[size++] = QUAD[i * 5 + j];
        quadSize = 0;
    }

    public static synchronized float[] collect(Object client, Object[] players, float tick,
            double cx, double cy, double cz) throws Exception {
        init(client.getClass().getClassLoader());
        size = 0; quadSize = 0;
        Object dispatcher = client.getClass().getMethod("method_1561").invoke(client);
        Object itemRenderer = client.getClass().getMethod("method_1480").invoke(client);
        for (Object player : players) {
            Object renderer = getRenderer.invoke(dispatcher, player);
            Object state = updateState.invoke(renderer, player, tick);
            Object model = getModel.invoke(renderer);
            setAngles.invoke(model, state);
            Class<?> stateClass = c("class_10017", loader);
            Object stack = stackCtor.newInstance();
            Object pos = offset.invoke(renderer, state);
            translate.invoke(stack,
                stateClass.getField("field_53325").getDouble(state) - cx + ((Number)vecX.invoke(pos)).doubleValue(),
                stateClass.getField("field_53326").getDouble(state) - cy + ((Number)vecY.invoke(pos)).doubleValue(),
                stateClass.getField("field_53327").getDouble(state) - cz + ((Number)vecZ.invoke(pos)).doubleValue());
            Object sleeping = sleepingDirection.get(state);
            if (sleeping != null) {
                double eye = eyeHeight.getFloat(state) - 0.1;
                translate.invoke(stack, -((int)directionX.invoke(sleeping)) * eye, 0.0,
                    -((int)directionZ.invoke(sleeping)) * eye);
            }
            float s = baseScale.getFloat(state);
            scale.invoke(stack, s, s, s);
            transforms.invoke(renderer, state, stack, bodyYaw.getFloat(state), s);
            scale.invoke(stack, -1.0f, -1.0f, 1.0f);
            rendererScale.invoke(renderer, state, stack);
            translate.invoke(stack, 0.0, -1.501, 0.0);
            Object entityWorld = player.getClass().getMethod("method_37908").invoke(player);
            boolean leftMain = leftArm.equals(player.getClass().getMethod("method_6068").invoke(player));
            renderHand(itemRenderer, player, model, stack, entityWorld, mainHand,
                leftMain ? leftArm : rightArm, leftMain ? thirdLeft : thirdRight, false);
            renderHand(itemRenderer, player, model, stack, entityWorld, offHand,
                leftMain ? rightArm : leftArm, leftMain ? thirdRight : thirdLeft, true);
        }
        return Arrays.copyOf(OUT, size);
    }
    private static void renderHand(Object renderer, Object player, Object model, Object stack,
            Object world, Object slot, Object arm, Object context, boolean left) throws Exception {
        Object held = player.getClass().getMethod("method_6118", slotClass).invoke(player, slot);
        if ((Boolean)held.getClass().getMethod("method_7960").invoke(held)) return;
        push.invoke(stack);
        try {
            translateToHand.invoke(model, arm, stack);
            translate.invoke(stack, 0.0, 0.125, 0.0);
            Object q = quaternionCtor.newInstance();
            q.getClass().getMethod("rotationX", float.class).invoke(q, (float)(-Math.PI / 2));
            q.getClass().getMethod("rotateY", float.class).invoke(q, (float)Math.PI);
            rotate.invoke(stack, q);
            itemRender.invoke(renderer, player, held, context, left, stack, BUFFER_PROVIDER,
                world, 0xF000F0, 0, 0);
        } finally { pop.invoke(stack); }
    }
}
