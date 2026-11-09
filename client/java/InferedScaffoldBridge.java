 





























import java.lang.reflect.*;
import java.util.concurrent.Executor;

 
public final class InferedScaffoldBridge {
    public static volatile boolean ACTIVE;
     
     
    public static volatile boolean AIM_ACTIVE, SHADOW_ACTIVE;
    public static volatile float AIM_YAW, AIM_PITCH, SHADOW_YAW, SHADOW_PITCH;
     
    public static volatile boolean BLOCKED, MENU_OPEN, AUTO_BLOCK = true, EDGE_SAFETY = true;
     


    public static volatile boolean SILENT_AIM;

     




    static boolean armBody(float yaw, float pitch) {
        if (bodyArmCapture) {
            bodyArmYaw = yaw;
            bodyArmPitch = pitch;
            bodyArmCalls++;
            return bodyArmResult;
        }
        return armBodyNative(yaw, pitch);
    }

    private static native boolean armBodyNative(float yaw, float pitch);

     

    static boolean bodyArmCapture, bodyArmResult = true;
    static int bodyArmCalls;
    static float bodyArmYaw, bodyArmPitch;

     






    private static volatile long settings = packSettings(0, 0, 4, 0, 10f, true, true);
     
    private static volatile float minDist = 0f;

     
     
     
    static Object client;
    static Method inputPhase, use, yaw, pitch, alive, spectator, riding, using, grounded, focused, paused;
    static Method held, empty, item, reach, positionX, positionY, positionZ, eye, vx, vy, vz;
    static Method hitKind, hitBlock, hitFace, air, raycast, collisionShape, shapeEmpty, shapeBoxes;
    static Method deltaMovement, setDeltaMovement, setPosition, boundingBox;
    static Method inventory, inventoryStack, stackCount, getBlock, defaultState, blockKey, fullCube;
    static Method jump, sneaking, updateContext, placementState, canPlace, placementPos, keyDown;
    static Field playerField, worldField, screenField, optionsField, pickField, targetField, cooldownField;
    static Field forwardKey, backKey, leftKey, rightKey, attackKey, useKey, pickKey, queuedClicks;
    static Field selectedSlot, jumpKey, shiftKey;
    static Method setKeyDown;
     


    private static boolean sneakHeld;
    private static Object sneakKey;
    static Field boxMinX, boxMinY, boxMinZ, boxMaxX, boxMaxY, boxMaxZ;
    static Class<?> blockHit, blockItemClass, fallingBlock;
    static Constructor<?> placementContext, vecConstructor, blockPos, rayContextConstructor;
    static Object shapeType, fluidMode, mainHand;
    static Method crystalInput;
    static boolean crystalInputResolved;

    private InferedScaffoldBridge() {}

    private static Class<?> c(ClassLoader loader, String name) throws Exception {
        return Class.forName("net.minecraft." + name, true, loader);
    }

    private static Method privateMethod(Class<?> type, String name, Class<?>... args) throws Exception {
        Method m = type.getDeclaredMethod(name, args);
        m.setAccessible(true);
        return m;
    }

    public static synchronized void configure(Object mc) throws Exception {
        releaseSneak();
        client = mc;
        ACTIVE = AIM_ACTIVE = SHADOW_ACTIVE = false;
        BLOCKED = MENU_OPEN = false;
        crystalInput = null;
        crystalInputResolved = false;
        InferedScaffoldLogic.reset();

        ClassLoader l = mc.getClass().getClassLoader();
        Class<?> entity = c(l, "class_1297"), living = c(l, "class_1309"), player = c(l, "class_1657");
        inputPhase = privateMethod(mc.getClass(), "method_1508");
        use = privateMethod(mc.getClass(), "method_1583");
        playerField = mc.getClass().getField("field_1724");
        worldField = mc.getClass().getField("field_1687");
        screenField = mc.getClass().getField("field_1755");
        optionsField = mc.getClass().getField("field_1690");
        pickField = mc.getClass().getField("field_1765");
        targetField = mc.getClass().getField("field_1692");
        cooldownField = mc.getClass().getDeclaredField("field_1752");
        cooldownField.setAccessible(true);
        yaw = entity.getMethod("method_36454");
        pitch = entity.getMethod("method_36455");
        alive = entity.getMethod("method_5805");
        spectator = player.getMethod("method_7325");
        riding = entity.getMethod("method_5765");
        using = living.getMethod("method_6115");
        grounded = entity.getMethod("method_24828");
        focused = mc.getClass().getMethod("method_1569");
        paused = mc.getClass().getMethod("method_1493");
        held = living.getMethod("method_6047");
        Class<?> stack = c(l, "class_1799");
        empty = stack.getMethod("method_7960");
        item = stack.getMethod("method_7909");
        blockItemClass = c(l, "class_1747");
        reach = player.getMethod("method_55754");
        positionX = entity.getMethod("method_23317");
        positionY = entity.getMethod("method_23318");
        positionZ = entity.getMethod("method_23321");
        eye = entity.getMethod("method_33571");
        boundingBox = entity.getMethod("method_5829");
        setPosition = entity.getMethod("method_5814", double.class, double.class, double.class);
        deltaMovement = entity.getMethod("method_18798");
        Class<?> vec = c(l, "class_243");
        setDeltaMovement = entity.getMethod("method_18799", vec);
        Class<?> aabb = c(l, "class_238");
        boxMinX = aabb.getField("field_1323");
        boxMinY = aabb.getField("field_1322");
        boxMinZ = aabb.getField("field_1321");
        boxMaxX = aabb.getField("field_1320");
        boxMaxY = aabb.getField("field_1325");
        boxMaxZ = aabb.getField("field_1324");
        vecConstructor = vec.getConstructor(double.class, double.class, double.class);
        vx = vec.getMethod("method_10216");
        vy = vec.getMethod("method_10214");
        vz = vec.getMethod("method_10215");
        Class<?> worldView = c(l, "class_1922"), pos = c(l, "class_2338"), rayContext = c(l, "class_3959");
        Class<?> shapeTypeClass = c(l, "class_3959$class_3960"), fluidTypeClass = c(l, "class_3959$class_242");
        shapeType = shapeTypeClass.getField("field_17558").get(null);
        fluidMode = fluidTypeClass.getField("field_1348").get(null);
        rayContextConstructor = rayContext.getConstructor(vec, vec, shapeTypeClass, fluidTypeClass, entity);
        raycast = worldView.getMethod("method_17742", rayContext);
        blockHit = c(l, "class_3965");
        hitKind = c(l, "class_239").getMethod("method_17783");
        hitBlock = blockHit.getMethod("method_17777");
        hitFace = blockHit.getMethod("method_17780");
        blockPos = pos.getConstructor(int.class, int.class, int.class);
        air = worldView.getMethod("method_8320", pos);
        Class<?> state = c(l, "class_4970$class_4971");
        collisionShape = state.getMethod("method_26220", worldView, pos);
        Class<?> shape = c(l, "class_265");
        shapeEmpty = shape.getMethod("method_1110");
        shapeBoxes = shape.getMethod("method_1090");
        Class<?> keys = c(l, "class_315");
        forwardKey = keys.getField("field_1894");
        backKey = keys.getField("field_1881");
        leftKey = keys.getField("field_1913");
        rightKey = keys.getField("field_1849");
        attackKey = keys.getField("field_1886");
        useKey = keys.getField("field_1904");
        pickKey = keys.getField("field_1871");
        keyDown = c(l, "class_304").getMethod("method_1434");
        queuedClicks = c(l, "class_304").getDeclaredField("field_1661");
        queuedClicks.setAccessible(true);
        jumpKey = keys.getField("field_1903");
        shiftKey = keys.getField("field_1832");
        setKeyDown = c(l, "class_304").getMethod("method_23481", boolean.class);
        jump = living.getMethod("method_6043");
        sneaking = privateMethod(player, "method_21825");
        inventory = player.getMethod("method_31548");
        Class<?> inv = c(l, "class_1661");
        inventoryStack = inv.getMethod("method_5438", int.class);
        selectedSlot = inv.getField("field_7545");
        stackCount = stack.getMethod("method_7947");
        getBlock = blockItemClass.getMethod("method_7711");
        Class<?> block = c(l, "class_2248");
        defaultState = block.getMethod("method_9564");
        blockKey = Object.class.getMethod("toString");
        fallingBlock = c(l, "class_2346");
        fullCube = state.getMethod("method_26234", worldView, pos);
        Class<?> context = c(l, "class_1750"), hand = c(l, "class_1268");
        placementContext = context.getConstructor(player, hand, stack, blockHit);
        mainHand = hand.getField("field_5808").get(null);
        updateContext = blockItemClass.getMethod("method_16356", context);
        placementState = privateMethod(blockItemClass, "method_7707", context);
        canPlace = privateMethod(blockItemClass, "method_7709", context, c(l, "class_2680"));
        placementPos = c(l, "class_1838").getMethod("method_8037");
    }

     

     
    private static long packSettings(int technique, int tower, int length, int delay, float speed,
                                     boolean auto, boolean safe) {
        int boundedTechnique = Math.max(0, Math.min(15, technique));
        int boundedTower = Math.max(0, Math.min(15, tower));
        int boundedLength = Math.max(1, Math.min(255, length));
        int boundedDelay = Math.max(0, Math.min(255, delay));
        float boundedSpeed = Float.isFinite(speed) ? Math.max(1, Math.min(45, speed)) : 10;
        return (Float.floatToIntBits(boundedSpeed) & 0xffffffffL)
            | ((long) boundedTechnique << 32)
            | ((long) boundedTower << 36)
            | ((long) boundedLength << 40)
            | ((long) boundedDelay << 48)
            | (auto ? 1L << 56 : 0)
            | (safe ? 1L << 57 : 0);
    }

     
    public static void settings(int technique, int tower, int length, int delay, float speed, float distance,
                                boolean auto, boolean safe) {
        minDist = Float.isFinite(distance) ? Math.max(0, Math.min(.25f, distance)) : 0f;
        settings = packSettings(technique, tower, length, delay, speed, auto, safe);
        AUTO_BLOCK = auto;
        EDGE_SAFETY = safe;
    }

    static long snapshot() {
        return settings;
    }

    static float minDistance() {
        return minDist;
    }

    static int technique(long value) {
        return (int) ((value >>> 32) & 15);
    }

    static int tower(long value) {
        return (int) ((value >>> 36) & 15);
    }

    static int expandLength(long value) {
        return (int) ((value >>> 40) & 255);
    }

    static int delay(long value) {
        return (int) ((value >>> 48) & 255);
    }

    static boolean autoBlock(long value) {
        return (value & (1L << 56)) != 0;
    }

    static boolean edgeSafety(long value) {
        return (value & (1L << 57)) != 0;
    }

    static float aimSpeed(long value) {
        return Float.intBitsToFloat((int) (value & 0xffffffffL));
    }

     

     



















    public static boolean safeWalk(Object p) {
        try {
            return (boolean) sneaking.invoke(p);
        } catch (Exception e) {
            throw new IllegalStateException("Scaffold edge check failed", cause(e));
        }
    }

     





    static void holdSneak(boolean want) throws Exception {
        if (client == null || optionsField == null || shiftKey == null
            || keyDown == null || setKeyDown == null) return;
        Object key = shiftKey.get(optionsField.get(client));
        if (key == null) return;
        if (sneakHeld && sneakKey != key) {
             
             
            setKeyDown.invoke(sneakKey, false);
            sneakHeld = false;
            sneakKey = null;
        }
        if (want) {
            if (!sneakHeld && !(boolean) keyDown.invoke(key)) {
                setKeyDown.invoke(key, true);
                sneakHeld = true;
                sneakKey = key;
            }
        } else if (sneakHeld) {
            setKeyDown.invoke(sneakKey, false);
            sneakHeld = false;
            sneakKey = null;
        }
    }

     

    static void releaseSneak() {
        try {
            holdSneak(false);
        } catch (Exception e) {
             
            sneakHeld = false;
            sneakKey = null;
        }
    }

     

    static boolean down(Field field) throws Exception {
        return (boolean) keyDown.invoke(field.get(optionsField.get(client)));
    }

    private static boolean manual(Field field) throws Exception {
        Object key = field.get(optionsField.get(client));
        return (boolean) keyDown.invoke(key) || queuedClicks.getInt(key) > 0;
    }

     

    static boolean clicking() throws Exception {
        return manual(attackKey) || manual(useKey) || manual(pickKey);
    }

    static boolean eligible(Object p) throws Exception {
        return ACTIVE && !BLOCKED && !MENU_OPEN && p != null && worldField.get(client) != null
            && screenField.get(client) == null && (boolean) focused.invoke(client) && !(boolean) paused.invoke(client)
            && (boolean) alive.invoke(p) && !(boolean) spectator.invoke(p) && !(boolean) riding.invoke(p)
            && !(boolean) using.invoke(p) && !manual(attackKey) && !manual(useKey) && !manual(pickKey);
    }

     

     

    public static void runInputPhase(Object mc) {
        if (mc != client) throw new IllegalStateException("Scaffold client mismatch");
        moduleInputPhase(mc);
        try {
            InferedScaffoldLogic.beforeInput();
        } catch (Throwable t) {
            InferedScaffoldLogic.failed(t);
        } finally {
            try {
                inputPhase.invoke(mc);
            } catch (Exception e) {
                throw new IllegalStateException("Vanilla input phase failed", cause(e));
            }
        }
    }

     

    private static void moduleInputPhase(Object mc) {
        if (!crystalInputResolved) {
            crystalInputResolved = true;
            try {
                crystalInput = Class.forName("InferedCrystalBridge", true, mc.getClass().getClassLoader())
                    .getMethod("beforeKeybinds", Object.class);
            } catch (Exception | LinkageError e) {
                crystalInput = null;
            }
        }
        Method input = crystalInput;
        if (input == null) return;
        try {
            input.invoke(null, mc);
        } catch (Throwable t) {
            crystalInput = null;
            System.err.println("[Scaffold] crystal input phase stopped: " + unwrap(t));
        }
    }

     

    public static float[] aim(double ex, double ey, double ez, double tx, double ty, double tz, double range) {
        double dx = tx - ex, dy = ty - ey, dz = tz - ez;
        double distance = dx * dx + dy * dy + dz * dz;
        if (!Double.isFinite(dx + dy + dz + range + distance) || range <= 0 || distance < 1e-12
            || distance > range * range) return null;
        return new float[] {
            (float) Math.toDegrees(Math.atan2(-dx, dz)),
            (float) -Math.toDegrees(Math.atan2(dy, Math.hypot(dx, dz)))
        };
    }

    public static double angularTurn(float currentYaw, float currentPitch, float targetYaw, float targetPitch) {
        double ya = Math.toRadians(currentYaw), pa = Math.toRadians(currentPitch);
        double yb = Math.toRadians(targetYaw), pb = Math.toRadians(targetPitch);
        double dot = Math.sin(ya) * Math.sin(yb) * Math.cos(pa) * Math.cos(pb)
            + Math.sin(pa) * Math.sin(pb)
            + Math.cos(ya) * Math.cos(yb) * Math.cos(pa) * Math.cos(pb);
        return Math.acos(Math.max(-1, Math.min(1, dot)));
    }

    public static float approach(float current, float goal, float limit) {
        float delta = (goal - current) % 360;
        if (delta > 180) delta -= 360;
        if (delta < -180) delta += 360;
        return current + Math.max(-limit, Math.min(limit, delta));
    }

    public static double[] worldDirection(float yaw, float forward, float left) {
        double a = Math.toRadians(yaw);
        double x = left * Math.cos(a) - forward * Math.sin(a), z = forward * Math.cos(a) + left * Math.sin(a);
        double length = Math.hypot(x, z);
        return length == 0 ? new double[] {0, 0} : new double[] {x / length, z / length};
    }

     

    public static void stop() {
        ACTIVE = false;
        AIM_ACTIVE = false;
        releaseSneak();
         
        if (client instanceof Executor executor) executor.execute(() -> {if (!ACTIVE) InferedScaffoldLogic.release();});
        else InferedScaffoldLogic.release();
    }

    static Throwable unwrap(Throwable t) {
        return t instanceof InvocationTargetException i ? i.getCause() : t;
    }

    static Throwable cause(Exception e) {
        return unwrap(e);
    }
}
