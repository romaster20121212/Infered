import java.lang.reflect.Method;

 




























public final class InferedSwingEchoBridge {
     
    public static final int CAPACITY = 32;

     
    public static volatile boolean ENABLED;
     



    public static volatile boolean FROZEN;
     
    public static volatile float DELAY_SECONDS = 0.2F;
     
    public static volatile int COPIES = 12;
     
    public static volatile float SPACING = 0.06F;
     
    public static volatile float HOLD_SECONDS = 0.0F;
     
    public static volatile float FADE_SECONDS = 0.45F;
     
    public static volatile float START_ALPHA = 1.0F;
     
    public static volatile float SMOOTHNESS = 0.0F;

     



    private static final int HISTORY = 4096;

    private static final float NANOS_PER_SECOND = 1.0E9F;

     
    private static final String BODY_METHOD = "method_3228";
    private static final int BODY_PARAMETERS = 10;
     
    private static final String RENDER_SYSTEM = "com.mojang.blaze3d.systems.RenderSystem";
     
    private static final String FLUSH_METHOD = "method_22993";

     
    private static final class Trail {
        final float[] swing = new float[CAPACITY];
        final long[] born = new long[CAPACITY];
        int count;
         
        float lastSpawn = -1.0F;

        void clear() {
            count = 0;
            lastSpawn = -1.0F;
        }

        void spawn(float progress, long now, int limit) {
             
             
            while (count >= limit) {
                System.arraycopy(swing, 1, swing, 0, count - 1);
                System.arraycopy(born, 1, born, 0, count - 1);
                count--;
            }
            swing[count] = progress;
            born[count] = now;
            count++;
            lastSpawn = progress;
        }

         
        void prune(long now) {
            int kept = 0;
            for (int i = 0; i < count; i++) {
                if (fade(born[i], now) < 1.0F) {
                    swing[kept] = swing[i];
                    born[kept] = born[i];
                    kept++;
                }
            }
            count = kept;
        }
    }

     
    private static final class History {
        final long[] at = new long[HISTORY];
        final float[] swing = new float[HISTORY];
         
        int start;
        int count;

        void clear() {
            start = 0;
            count = 0;
        }

        void record(long now, float progress) {
            int index;
            if (count < HISTORY) {
                index = (start + count) % HISTORY;
                count++;
            } else {
                index = start;
                start = (start + 1) % HISTORY;
            }
            at[index] = now;
            swing[index] = progress;
        }

         





        boolean sample(long target, float[] result) {
            if (count == 0) {
                return false;
            }
            int newest = (start + count - 1) % HISTORY;
            if (at[newest] <= target) {
                result[0] = swing[newest];
                return true;
            }
            for (int i = 1; i < count; i++) {
                int newer = (start + count - i) % HISTORY;
                int older = (start + count - i - 1) % HISTORY;
                if (at[older] <= target) {
                    float span = (float) (at[newer] - at[older]);
                    float t = span > 0.0F ? (float) (target - at[older]) / span : 1.0F;
                    float before = swing[older];
                    float after = swing[newer];
                     
                    result[0] = after < before ? before : before + (after - before) * t;
                    return true;
                }
            }
            return false;
        }
    }

    private static final Trail MAIN = new Trail();
    private static final Trail OFF = new Trail();
    private static final History MAIN_HISTORY = new History();
    private static final History OFF_HISTORY = new History();
     
    private static final float[] SAMPLE = new float[1];

     
    static boolean replaying;
     
    private static boolean reported;

     
    private static Class<?> cachedOwner;
    private static Method cachedBody;
    private static Method cachedShaderColor;
    private static Class<?> cachedFlushOwner;
    private static Method cachedFlush;

    private InferedSwingEchoBridge() {}

     







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
         
        if (replaying || InferedHandOutlineBridge.replaying) {
            return;
        }
        if (!ENABLED || self == null || matrices == null) {
            MAIN.clear();
            OFF.clear();
            MAIN_HISTORY.clear();
            OFF_HISTORY.clear();
            return;
        }
        boolean off = isOffHand(hand);
        Trail trail = off ? OFF : MAIN;
        History history = off ? OFF_HISTORY : MAIN_HISTORY;
        float progress = swing instanceof Float ? (Float) swing : 0.0F;
        long now = System.nanoTime();
        history.record(now, progress);
        try {
            if (FROZEN) {
                recordFrozen(trail, progress, now);
                if (trail.count == 0) {
                    return;
                }
                replay(trail, now, self, player, tickDelta, pitch, hand, item, equip,
                        matrices, vc, light);
            } else {
                drawDelayed(history, progress, now, self, player, tickDelta, pitch, hand,
                        item, equip, matrices, vc, light);
            }
        } catch (ReflectiveOperationException | RuntimeException error) {
            trail.clear();
            history.clear();
            if (!reported) {
                reported = true;
                System.err.println("Swing Echo: copies are not drawn: " + error);
            }
        }
    }

     
    private static void recordFrozen(Trail trail, float progress, long now) {
        int limit = Math.max(1, Math.min(COPIES, CAPACITY));
        if (!(progress > 0.0F)) {
             
             
            trail.lastSpawn = -1.0F;
        } else {
            if (progress < trail.lastSpawn) {
                 
                trail.lastSpawn = -1.0F;
            }
            if (trail.lastSpawn < 0.0F || progress - trail.lastSpawn >= SPACING) {
                trail.spawn(progress, now, limit);
            }
        }
        trail.prune(now);
    }

     




    private static void drawDelayed(
            History history,
            float progress,
            long now,
            Object self,
            Object player,
            Object tickDelta,
            Object pitch,
            Object hand,
            Object item,
            Object equip,
            Object matrices,
            Object vc,
            Object light)
            throws ReflectiveOperationException {
        float delay = clamp(DELAY_SECONDS, 0.0F, 2.0F);
         
        if (!(delay > 0.0F)) {
            return;
        }
        long target = now - (long) (delay * NANOS_PER_SECOND);
        if (!history.sample(target, SAMPLE)) {
            return;
        }
        if (!(SAMPLE[0] > 0.0F) && !(progress > 0.0F)) {
            return;
        }
        Method shader = shaderColor(self.getClass());
        replaying = true;
        try {
            draw(SAMPLE[0], clamp01(START_ALPHA), self, player, tickDelta, pitch, hand, item,
                    equip, matrices, vc, light);
        } finally {
            replaying = false;
            shader.invoke(null, 1.0F, 1.0F, 1.0F, 1.0F);
        }
    }

     
    private static void replay(
            Trail trail,
            long now,
            Object self,
            Object player,
            Object tickDelta,
            Object pitch,
            Object hand,
            Object item,
            Object equip,
            Object matrices,
            Object vc,
            Object light)
            throws ReflectiveOperationException {
        Method shader = shaderColor(self.getClass());
        replaying = true;
        try {
            for (int i = 0; i < trail.count; i++) {
                float alpha = clamp01(START_ALPHA) * (1.0F - fade(trail.born[i], now));
                draw(trail.swing[i], alpha, self, player, tickDelta, pitch, hand, item, equip,
                        matrices, vc, light);
            }
        } finally {
            replaying = false;
            shader.invoke(null, 1.0F, 1.0F, 1.0F, 1.0F);
        }
    }

     







    private static void draw(
            float swingProgress,
            float alpha,
            Object self,
            Object player,
            Object tickDelta,
            Object pitch,
            Object hand,
            Object item,
            Object equip,
            Object matrices,
            Object vc,
            Object light)
            throws ReflectiveOperationException {
         
        InferedHandOutlineBridge.outlinePose(
                self, player, tickDelta, pitch, hand, swingProgress, item, equip, matrices, vc,
                light);
        Method body = bodyMethod(self.getClass());
        Method shader = shaderColor(self.getClass());
        Method flush = flushMethod(vc);
        shader.invoke(null, 1.0F, 1.0F, 1.0F, alpha);
        body.invoke(
                self,
                player,
                tickDelta,
                pitch,
                hand,
                swingProgress,
                item,
                equip,
                matrices,
                vc,
                light);
        flush.invoke(vc);
    }

     



    private static float fade(long born, long now) {
        float age = (now - born) / NANOS_PER_SECOND;
        if (age <= HOLD_SECONDS) {
            return 0.0F;
        }
        float t = FADE_SECONDS > 0.0F ? clamp01((age - HOLD_SECONDS) / FADE_SECONDS) : 1.0F;
        float eased = t * t * (3.0F - 2.0F * t);
        return t + (eased - t) * clamp01(SMOOTHNESS);
    }

    private static float clamp01(float value) {
        return clamp(value, 0.0F, 1.0F);
    }

    private static float clamp(float value, float min, float max) {
        if (value != value) {
            return min;
        }
        return value < min ? min : (value > max ? max : value);
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

     




    private static Method flushMethod(Object vc) throws ReflectiveOperationException {
        if (vc == null) {
            throw new NoSuchMethodException("no vertex consumer provider to flush");
        }
        Class<?> owner = vc.getClass();
        if (owner != cachedFlushOwner || cachedFlush == null) {
            Method flush = owner.getMethod(FLUSH_METHOD);
            flush.setAccessible(true);
            cachedFlushOwner = owner;
            cachedFlush = flush;
        }
        return cachedFlush;
    }

    private static boolean isOffHand(Object hand) {
        return hand instanceof Enum && ((Enum<?>) hand).name().contains("OFF_HAND");
    }
}
