import java.lang.reflect.*;

 















public final class InferedSkeletonBridge {
    public static final int MAX_PLAYERS = 40;
     
    public static final int JOINTS = 12;
    public static final int PLAYER_FLOATS = JOINTS * 3;
    public static final int MAX_FLOATS = MAX_PLAYERS * PLAYER_FLOATS;

    private static ClassLoader gameLoader;
    private static Method getRenderer, updateState, getModel, setAngles, rootPart;
    private static Method translate, scale, transforms, rendererScale, offset;
    private static Method pushPose, popPose, last, poseOf, translateAndRotate, matrixGet;
    private static Constructor<?> matrixStack;
    private static Field baseScale, bodyYaw, sleepDirection, eyeHeight;
    private static Field head, body, rightArm, leftArm, rightLeg, leftLeg;
    private static Method directionX, directionZ, vecX, vecY, vecZ;
    private static final float[] joints = new float[MAX_FLOATS];
    private static int floats;

    private InferedSkeletonBridge() {}

    private static void init(ClassLoader cl) throws Exception {
        if (gameLoader == cl) return;
        Class<?> entity = Class.forName("net.minecraft.class_1297", true, cl);
        Class<?> dispatcher = Class.forName("net.minecraft.class_898", true, cl);
        Class<?> renderer = Class.forName("net.minecraft.class_897", true, cl);
        Class<?> living = Class.forName("net.minecraft.class_922", true, cl);
        Class<?> state = Class.forName("net.minecraft.class_10017", true, cl);
        Class<?> livingState = Class.forName("net.minecraft.class_10042", true, cl);
        Class<?> stack = Class.forName("net.minecraft.class_4587", true, cl);
        Class<?> entry = Class.forName("net.minecraft.class_4587$class_4665", true, cl);
        Class<?> model = Class.forName("net.minecraft.class_583", true, cl);
        Class<?> part = Class.forName("net.minecraft.class_630", true, cl);
        Class<?> base = Class.forName("net.minecraft.class_3879", true, cl);
        Class<?> humanoid = Class.forName("net.minecraft.class_572", true, cl);
        Class<?> direction = Class.forName("net.minecraft.class_2350", true, cl);
        Class<?> vec = Class.forName("net.minecraft.class_243", true, cl);

        getRenderer = dispatcher.getMethod("method_3953", entity);
        updateState = renderer.getMethod("method_62425", entity, float.class);
        getModel = living.getMethod("method_4038");
        setAngles = model.getMethod("method_2819", state);
         
         
        try {
            rootPart = base.getMethod("method_63512");
        } catch (NoSuchMethodException ignored) {
            rootPart = null;
        }
        translate = stack.getMethod("method_22904", double.class, double.class, double.class);
        scale = stack.getMethod("method_22905", float.class, float.class, float.class);
        pushPose = stack.getMethod("method_22903");
        popPose = stack.getMethod("method_22909");
        last = stack.getMethod("method_23760");
        poseOf = entry.getMethod("method_23761");
        translateAndRotate = part.getMethod("method_22703", stack);
        matrixStack = stack.getConstructor();
        transforms = living.getDeclaredMethod("method_4058", livingState, stack, float.class, float.class);
        transforms.setAccessible(true);
        rendererScale = living.getDeclaredMethod("method_4042", livingState, stack);
        rendererScale.setAccessible(true);
        offset = renderer.getMethod("method_23169", state);
        baseScale = livingState.getField("field_53453");
        bodyYaw = livingState.getField("field_53446");
        sleepDirection = livingState.getField("field_53463");
        eyeHeight = state.getField("field_53331");
        head = humanoid.getField("field_3398");
        body = humanoid.getField("field_3391");
        rightArm = humanoid.getField("field_3401");
        leftArm = humanoid.getField("field_27433");
        rightLeg = humanoid.getField("field_3392");
        leftLeg = humanoid.getField("field_3397");
        directionX = direction.getMethod("method_10148");
        directionZ = direction.getMethod("method_10165");
        vecX = vec.getMethod("method_10216");
        vecY = vec.getMethod("method_10214");
        vecZ = vec.getMethod("method_10215");
        matrixGet = Class.forName("org.joml.Matrix4f", true, cl).getMethod("get", float[].class);
        gameLoader = cl;
    }

     
    private static void emitTranslation(Object stack) throws Exception {
        Object pose = last.invoke(stack);
        float[] matrix = (float[]) matrixGet.invoke(poseOf.invoke(pose), (Object) new float[16]);
        joints[floats++] = matrix[12];
        joints[floats++] = matrix[13];
        joints[floats++] = matrix[14];
    }

     






    private static void emitBone(Object stack, Object part, float tipY) throws Exception {
        pushPose.invoke(stack);
        translateAndRotate.invoke(part, stack);
        emitTranslation(stack);
        translate.invoke(stack, 0.0, tipY / 16.0, 0.0);
        emitTranslation(stack);
        popPose.invoke(stack);
    }

     






    public static synchronized float[] collect(
            Object client, Object[] players, float tick, double cx, double cy, double cz) throws Exception {
        init(client.getClass().getClassLoader());
        Object dispatcher = client.getClass().getMethod("method_1561").invoke(client);
        floats = 0;
        for (int i = 0; i < Math.min(players.length, MAX_PLAYERS); i++) {
            if (floats + PLAYER_FLOATS > joints.length) break;
            int mark = floats;
            try {
                appendPlayer(dispatcher, players[i], tick, cx, cy, cz);
            } catch (ReflectiveOperationException | IllegalStateException | NullPointerException error) {
                 
                floats = mark;
            }
        }
        return java.util.Arrays.copyOf(joints, floats);
    }

    private static void appendPlayer(
            Object dispatcher, Object player, float tick, double cx, double cy, double cz) throws Exception {
        Object renderer = getRenderer.invoke(dispatcher, player);
        Object state = updateState.invoke(renderer, player, tick);
        Object stack = matrixStack.newInstance();
        Class<?> stateClass = Class.forName("net.minecraft.class_10017", true, gameLoader);
        Object positionOffset = offset.invoke(renderer, state);
        double ox = (double) vecX.invoke(positionOffset);
        double oy = (double) vecY.invoke(positionOffset);
        double oz = (double) vecZ.invoke(positionOffset);
        translate.invoke(
                stack,
                stateClass.getField("field_53325").getDouble(state) - cx + ox,
                stateClass.getField("field_53326").getDouble(state) - cy + oy,
                stateClass.getField("field_53327").getDouble(state) - cz + oz);
         
        Object sleeping = sleepDirection.get(state);
        if (sleeping != null) {
            float eye = eyeHeight.getFloat(state) - 0.1f;
            translate.invoke(
                    stack,
                    -((int) directionX.invoke(sleeping)) * (double) eye,
                    0.0,
                    -((int) directionZ.invoke(sleeping)) * (double) eye);
        }
        float s = baseScale.getFloat(state);
        scale.invoke(stack, s, s, s);
        transforms.invoke(renderer, state, stack, bodyYaw.getFloat(state), s);
        scale.invoke(stack, -1.0f, -1.0f, 1.0f);
        rendererScale.invoke(renderer, state, stack);
        translate.invoke(stack, 0.0, -1.501, 0.0);
        Object model = getModel.invoke(renderer);
        setAngles.invoke(model, state);

        Object root = rootPart == null ? null : rootPart.invoke(model);
        int mark = floats;
        pushPose.invoke(stack);
        try {
            if (root != null) translateAndRotate.invoke(root, stack);
             
             
             
            emitBone(stack, head.get(model), -8.0f);
            emitBone(stack, body.get(model), 12.0f);
            emitBone(stack, rightArm.get(model), 10.0f);
            emitBone(stack, leftArm.get(model), 10.0f);
            emitBone(stack, rightLeg.get(model), 12.0f);
            emitBone(stack, leftLeg.get(model), 12.0f);
        } catch (ReflectiveOperationException | NullPointerException error) {
            floats = mark;
            throw new IllegalStateException("humanoid model part missing", error);
        } finally {
            popPose.invoke(stack);
        }
        if (floats != mark + PLAYER_FLOATS) {
            floats = mark;
            throw new IllegalStateException("incomplete skeleton emission");
        }
    }

    public static synchronized void clear() {
        floats = 0;
    }
}
