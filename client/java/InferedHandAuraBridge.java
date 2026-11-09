import java.lang.reflect.Method;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.IntBuffer;

 














































public final class InferedHandAuraBridge {
     
    public static volatile boolean ENABLED;
     
    public static volatile boolean MAIN_HAND = true;
    public static volatile boolean OFF_HAND = true;
     




    public static volatile float RED = 1.0F;
    public static volatile float GREEN = 0.0F;
    public static volatile float BLUE = 0.0F;
    public static volatile float ALPHA = 0.75F;
     
    public static volatile float WIDTH = 6.0F;
     







    public static volatile boolean ECHO_BLEED = true;

    private static final float MAX_WIDTH = 32.0F;
     
    private static final float ECHO_HALF_LIFE = 0.16F;
     
    private static final double MAX_ECHO_STEP = 0.25D;

     
    private static boolean reported;
     
    private static boolean auraWasEnabled;
     




    private static ClassLoader loader;
     
    private static boolean resourcesFailed;

    private static int framebuffer;
    private static int colorTexture;
    private static int maskTexture;
    private static int glowFramebuffer;
    private static int glowFramebufferB;
    private static int glowTextureA;
    private static int glowTextureB;
    private static int vertexArray;

     



    private static int echoFramebufferA;
    private static int echoFramebufferB;
    private static int echoTextureA;
    private static int echoTextureB;
     
    private static boolean echoPrimed;
     
    private static long lastFrameNanos;

     
    private static int extractProgram;
    private static int extractMaskUniform;
    private static int extractTexelUniform;
    private static int extractRadiusUniform;

     
    private static int accumulateProgram;
    private static int accumulateCurrentUniform;
    private static int accumulateEchoUniform;
    private static int accumulateDecayUniform;

     
    private static int blurProgram;
    private static int blurSourceUniform;
    private static int blurDirectionUniform;
    private static int blurSigmaUniform;

     
    private static int compositeProgram;
    private static int compositeMaskUniform;
    private static int compositeGlowUniform;
    private static int compositeColorUniform;

    private static int width;
    private static int height;
    private static int glowWidth;
    private static int glowHeight;

    private static final String VERTEX_SHADER =
            "#version 150\n"
                    + "out vec2 uv;\n"
                    + "void main() {\n"
                    + "    vec2 at = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));\n"
                    + "    uv = at;\n"
                     
                     
                    + "    gl_Position = vec4(at * 2.0 - 1.0, 1.0, 1.0);\n"
                    + "}\n";

     






    private static final String EXTRACT_SHADER =
            "#version 150\n"
                    + "uniform sampler2D Mask;\n"
                    + "uniform vec2 Texel;\n"
                    + "uniform float Radius;\n"
                    + "in vec2 uv;\n"
                    + "out vec4 fragColor;\n"
                    + "const int TAPS = 32;\n"
                    + "void main() {\n"
                     
                     
                     
                    + "    if (texture(Mask, uv).r < 0.999999) {\n"
                    + "        fragColor = vec4(0.0);\n"
                    + "        return;\n"
                    + "    }\n"
                     
                     
                     
                     
                    + "    float nearest = 0.0;\n"
                    + "    for (int i = 0; i < TAPS; i++) {\n"
                    + "        float fi = float(i);\n"
                    + "        float angle = fi * 2.39996323;\n"
                    + "        float reach = sqrt((fi + 0.5) / float(TAPS));\n"
                    + "        vec2 offset = vec2(cos(angle), sin(angle)) * (reach * Radius) * Texel;\n"
                    + "        if (texture(Mask, uv + offset).r < 0.999999) {\n"
                    + "            nearest = max(nearest, 1.0 - reach);\n"
                    + "        }\n"
                    + "    }\n"
                    + "    float away = 1.0 - nearest;\n"
                     
                     
                     
                    + "    if (nearest <= 0.0) {\n"
                    + "        fragColor = vec4(0.0);\n"
                    + "        return;\n"
                    + "    }\n"
                     
                     
                    + "    float core = 1.0 - smoothstep(0.0, 2.0, away * Radius);\n"
                    + "    float halo = pow(1.0 - away, 1.7) * exp(-1.9 * away);\n"
                    + "    float glow = clamp(core * 1.5 + halo * 1.1, 0.0, 1.0);\n"
                    + "    fragColor = vec4(glow, glow, glow, 1.0);\n"
                    + "}\n";

     
    private static final String BLUR_SHADER =
            "#version 150\n"
                    + "uniform sampler2D Source;\n"
                    + "uniform vec2 Direction;\n"
                    + "uniform float Sigma;\n"
                    + "in vec2 uv;\n"
                    + "out vec4 fragColor;\n"
                    + "void main() {\n"
                    + "    float sigma = max(Sigma, 0.5);\n"
                    + "    float stepSize = sigma / 3.0;\n"
                    + "    vec4 sum = vec4(0.0);\n"
                    + "    float weightSum = 0.0;\n"
                    + "    for (int i = -6; i <= 6; ++i) {\n"
                    + "        float x = float(i) * stepSize;\n"
                    + "        float w = exp(-0.5 * x * x / (sigma * sigma));\n"
                    + "        sum += texture(Source, uv + Direction * x) * w;\n"
                    + "        weightSum += w;\n"
                    + "    }\n"
                    + "    fragColor = sum / weightSum;\n"
                    + "}\n";

     











    private static final String ACCUMULATE_SHADER =
            "#version 150\n"
                    + "uniform sampler2D Current;\n"
                    + "uniform sampler2D Echo;\n"
                    + "uniform float Decay;\n"
                    + "in vec2 uv;\n"
                    + "out vec4 fragColor;\n"
                    + "void main() {\n"
                    + "    float current = texture(Current, uv).r;\n"
                     
                     
                     
                    + "    vec2 texel = 1.0 / vec2(textureSize(Current, 0));\n"
                    + "    float near = 0.0;\n"
                    + "    for (int i = 0; i < 8; ++i) {\n"
                    + "        float angle = float(i) * 0.7853981634;\n"
                    + "        near = max(near, texture(Current, uv + vec2(cos(angle), sin(angle)) * texel * 5.0).r);\n"
                    + "    }\n"
                     
                     
                    + "    float band = smoothstep(0.015, 0.12, max(current, near));\n"
                     
                     
                    + "    float echo = texture(Echo, uv).r * Decay * 0.85 * band;\n"
                     
                    + "    fragColor = vec4(max(current, echo), 0.0, 0.0, 1.0);\n"
                    + "}\n";

     

    private static final String COMPOSITE_SHADER =
            "#version 150\n"
                    + "uniform sampler2D Mask;\n"
                    + "uniform sampler2D Glow;\n"
                    + "uniform vec4 Color;\n"
                    + "in vec2 uv;\n"
                    + "out vec4 fragColor;\n"
                    + "void main() {\n"
                    + "    if (texture(Mask, uv).r < 0.999999) {\n"
                    + "        fragColor = vec4(0.0);\n"
                    + "        return;\n"
                    + "    }\n"
                    + "    float glow = texture(Glow, uv).r;\n"
                     
                     
                     
                     
                    + "    if (glow <= 0.002) {\n"
                    + "        fragColor = vec4(0.0);\n"
                    + "        return;\n"
                    + "    }\n"
                    + "    float shaped = 1.0 - exp(-glow * 3.4);\n"
                     
                     
                    + "    float amount = clamp(shaped * Color.a * 1.35, 0.0, 1.0);\n"
                    + "    fragColor = vec4(Color.rgb * amount, 0.0);\n"
                    + "}\n";

    private InferedHandAuraBridge() {}

     










    public static void afterHands(Object self) {
        if (!ENABLED) {
            auraWasEnabled = false;
            echoPrimed = false;
            lastFrameNanos = 0L;
            return;
        }
        if (!auraWasEnabled) {
             
             
            auraWasEnabled = true;
            resourcesFailed = false;
             
             
            echoPrimed = false;
            lastFrameNanos = 0L;
        }
        if (self != null) {
            loader = self.getClass().getClassLoader();
        }
        ClassLoader owner = loader;
        if (owner == null || !MAIN_HAND && !OFF_HAND) {
            return;
        }
        Look look = new Look(RED, GREEN, BLUE, ALPHA, WIDTH, ECHO_BLEED);
         
        if (!(look.width() > 0.0F)) {
            echoPrimed = false;
            lastFrameNanos = 0L;
            return;
        }
         
         
        if (!look.echo()) {
            echoPrimed = false;
        }
        float decay = echoDecay(System.nanoTime(), look.echo());
        try {
            Gl.load(owner);
            Gl.State state = Gl.State.capture(owner);
            try {
                if (!ensureResources(state.viewportBox[2], state.viewportBox[3])) {
                    return;
                }
                 
                 
                 
                if (!Gl.copyDepth(state, framebuffer)) {
                    return;
                }
                addGlow(state, look, decay);
            } finally {
                state.restore(owner);
            }
        } catch (ReflectiveOperationException | RuntimeException error) {
            if (!reported) {
                reported = true;
                System.err.println("Hand Aura: aura is not drawn: " + root(error));
            }
        }
    }

     







    private static void addGlow(Gl.State state, Look look, float decay)
            throws ReflectiveOperationException {
        float reach = look.width();
        float alpha = look.alpha();
        Gl.beginGlow(glowFramebuffer);
        try {
            Gl.extract(
                    maskTexture,
                    extractProgram,
                    extractMaskUniform,
                    extractTexelUniform,
                    extractRadiusUniform,
                    vertexArray,
                    width,
                    height,
                    reach);
            float sigma = Math.max(0.7F, reach * 0.22F);
            Gl.blur(
                    glowFramebufferB,
                    glowTextureA,
                    blurProgram,
                    blurSourceUniform,
                    blurDirectionUniform,
                    blurSigmaUniform,
                    vertexArray,
                    glowWidth,
                    glowHeight,
                    sigma,
                    true);
            Gl.blur(
                    glowFramebuffer,
                    glowTextureB,
                    blurProgram,
                    blurSourceUniform,
                    blurDirectionUniform,
                    blurSigmaUniform,
                    vertexArray,
                    glowWidth,
                    glowHeight,
                    sigma,
                    false);
        } finally {
            Gl.endGlow(state);
        }
        int glow = glowTextureA;
        if (look.echo()) {
             
            Gl.beginGlow(echoFramebufferB);
            try {
                Gl.accumulate(
                        glowTextureA,
                        echoTextureA,
                        accumulateProgram,
                        accumulateCurrentUniform,
                        accumulateEchoUniform,
                        accumulateDecayUniform,
                        vertexArray,
                        decay);
            } finally {
                Gl.endGlow(state);
            }
             
            int texture = echoTextureA;
            echoTextureA = echoTextureB;
            echoTextureB = texture;
            int framebuffer = echoFramebufferA;
            echoFramebufferA = echoFramebufferB;
            echoFramebufferB = framebuffer;
            echoPrimed = true;
            glow = echoTextureA;
        }
        Gl.composite(
                maskTexture,
                glow,
                compositeProgram,
                compositeMaskUniform,
                compositeGlowUniform,
                compositeColorUniform,
                vertexArray,
                look.red(),
                look.green(),
                look.blue(),
                alpha);
    }

     
    private static final class Look {
        final float red;
        final float green;
        final float blue;
        final float alpha;
        final float width;
        final boolean echo;

        Look(float red, float green, float blue, float alpha, float width, boolean echo) {
            this.red = red;
            this.green = green;
            this.blue = blue;
            this.alpha = alpha;
            this.width = width;
            this.echo = echo;
        }

         
        boolean echo() {
            return echo;
        }

         
        float width() {
            return finite(width, 0.0F, MAX_WIDTH, 5.0F);
        }

        float red() {
            return finite(red, 0.0F, 1.0F, 1.0F);
        }

        float green() {
            return finite(green, 0.0F, 1.0F, 0.0F);
        }

        float blue() {
            return finite(blue, 0.0F, 1.0F, 0.0F);
        }

         
        float alpha() {
            return finite(alpha, 0.0F, 1.0F, 0.5F);
        }
    }

     




    private static boolean ensureResources(int targetWidth, int targetHeight) {
        if (resourcesFailed) {
            return false;
        }
        if (extractProgram != 0 && width == targetWidth && height == targetHeight) {
            return true;
        }
        release();
        try {
            Gl.call(Gl.activeTexture, Gl.TEXTURE0);
            int glowTargetWidth = Math.max(1, targetWidth / 2);
            int glowTargetHeight = Math.max(1, targetHeight / 2);

            extractProgram = linkProgram(EXTRACT_SHADER);
            extractMaskUniform = Gl.uniform(extractProgram, "Mask");
            extractTexelUniform = Gl.uniform(extractProgram, "Texel");
            extractRadiusUniform = Gl.uniform(extractProgram, "Radius");
            if (extractMaskUniform == -1
                    || extractTexelUniform == -1
                    || extractRadiusUniform == -1) {
                throw new IllegalStateException("missing extract uniforms");
            }

            blurProgram = linkProgram(BLUR_SHADER);
            blurSourceUniform = Gl.uniform(blurProgram, "Source");
            blurDirectionUniform = Gl.uniform(blurProgram, "Direction");
            blurSigmaUniform = Gl.uniform(blurProgram, "Sigma");
            if (blurSourceUniform == -1
                    || blurDirectionUniform == -1
                    || blurSigmaUniform == -1) {
                throw new IllegalStateException("missing blur uniforms");
            }

            accumulateProgram = linkProgram(ACCUMULATE_SHADER);
            accumulateCurrentUniform = Gl.uniform(accumulateProgram, "Current");
            accumulateEchoUniform = Gl.uniform(accumulateProgram, "Echo");
            accumulateDecayUniform = Gl.uniform(accumulateProgram, "Decay");
            if (accumulateCurrentUniform == -1
                    || accumulateEchoUniform == -1
                    || accumulateDecayUniform == -1) {
                throw new IllegalStateException("missing accumulate uniforms");
            }

            compositeProgram = linkProgram(COMPOSITE_SHADER);
            compositeMaskUniform = Gl.uniform(compositeProgram, "Mask");
            compositeGlowUniform = Gl.uniform(compositeProgram, "Glow");
            compositeColorUniform = Gl.uniform(compositeProgram, "Color");
            if (compositeMaskUniform == -1
                    || compositeGlowUniform == -1
                    || compositeColorUniform == -1) {
                throw new IllegalStateException("missing composite uniforms");
            }

            vertexArray = Gl.genVertexArray();

            colorTexture = Gl.genTexture();
            Gl.bindTexture(colorTexture);
            Gl.texImage2D(targetWidth, targetHeight, false);
            Gl.textureParameters(Gl.NEAREST);

            maskTexture = Gl.genTexture();
            Gl.bindTexture(maskTexture);
            Gl.texImage2D(targetWidth, targetHeight, true);
             
             
             
             
            Gl.textureParameters(Gl.NEAREST);

            framebuffer = Gl.genFramebuffer();
            Gl.bindFramebuffer(Gl.FRAMEBUFFER, framebuffer);
            Gl.framebufferTexture(Gl.COLOR_ATTACHMENT0, colorTexture);
            Gl.framebufferTexture(Gl.DEPTH_ATTACHMENT, maskTexture);
            Gl.requireComplete();

            glowTextureA = Gl.genTexture();
            Gl.bindTexture(glowTextureA);
            Gl.texImage2D(glowTargetWidth, glowTargetHeight, false);
            Gl.textureParameters(Gl.LINEAR);

            glowTextureB = Gl.genTexture();
            Gl.bindTexture(glowTextureB);
            Gl.texImage2D(glowTargetWidth, glowTargetHeight, false);
            Gl.textureParameters(Gl.LINEAR);

            glowFramebuffer = Gl.genFramebuffer();
            Gl.bindFramebuffer(Gl.FRAMEBUFFER, glowFramebuffer);
            Gl.framebufferTexture(Gl.COLOR_ATTACHMENT0, glowTextureA);
            Gl.requireComplete();

            glowFramebufferB = Gl.genFramebuffer();
            Gl.bindFramebuffer(Gl.FRAMEBUFFER, glowFramebufferB);
            Gl.framebufferTexture(Gl.COLOR_ATTACHMENT0, glowTextureB);
            Gl.requireComplete();

            echoTextureA = Gl.genTexture();
            Gl.bindTexture(echoTextureA);
            Gl.texImage2D(glowTargetWidth, glowTargetHeight, false);
            Gl.textureParameters(Gl.LINEAR);

            echoTextureB = Gl.genTexture();
            Gl.bindTexture(echoTextureB);
            Gl.texImage2D(glowTargetWidth, glowTargetHeight, false);
            Gl.textureParameters(Gl.LINEAR);

            echoFramebufferA = Gl.genFramebuffer();
            Gl.bindFramebuffer(Gl.FRAMEBUFFER, echoFramebufferA);
            Gl.framebufferTexture(Gl.COLOR_ATTACHMENT0, echoTextureA);
            Gl.requireComplete();

            echoFramebufferB = Gl.genFramebuffer();
            Gl.bindFramebuffer(Gl.FRAMEBUFFER, echoFramebufferB);
            Gl.framebufferTexture(Gl.COLOR_ATTACHMENT0, echoTextureB);
            Gl.requireComplete();

             
            echoPrimed = false;
            lastFrameNanos = 0L;

            width = targetWidth;
            height = targetHeight;
            glowWidth = glowTargetWidth;
            glowHeight = glowTargetHeight;
            return true;
        } catch (ReflectiveOperationException | RuntimeException error) {
            resourcesFailed = true;
            release();
            if (!reported) {
                reported = true;
                System.err.println("Hand Aura: outline is not drawn: " + root(error));
            }
            return false;
        }
    }

     
    private static void release() {
        if (extractProgram == 0
                && blurProgram == 0
                && accumulateProgram == 0
                && compositeProgram == 0
                && framebuffer == 0
                && glowFramebuffer == 0
                && glowFramebufferB == 0
                && echoFramebufferA == 0
                && echoFramebufferB == 0
                && maskTexture == 0
                && colorTexture == 0
                && glowTextureA == 0
                && glowTextureB == 0
                && echoTextureA == 0
                && echoTextureB == 0
                && vertexArray == 0) {
            return;
        }
        try {
            Gl.release(
                    new int[] {extractProgram, blurProgram, accumulateProgram, compositeProgram},
                    vertexArray,
                    new int[] {
                        framebuffer, glowFramebuffer, glowFramebufferB, echoFramebufferA, echoFramebufferB
                    },
                    new int[] {
                        maskTexture, colorTexture, glowTextureA, glowTextureB, echoTextureA, echoTextureB
                    });
        } catch (RuntimeException error) {
            System.err.println("Hand Aura: cleanup failed: " + root(error));
        }
        extractProgram = 0;
        blurProgram = 0;
        accumulateProgram = 0;
        compositeProgram = 0;
        vertexArray = 0;
        framebuffer = 0;
        glowFramebuffer = 0;
        glowFramebufferB = 0;
        echoFramebufferA = 0;
        echoFramebufferB = 0;
        maskTexture = 0;
        colorTexture = 0;
        glowTextureA = 0;
        glowTextureB = 0;
        echoTextureA = 0;
        echoTextureB = 0;
        echoPrimed = false;
        lastFrameNanos = 0L;
        width = 0;
        height = 0;
        glowWidth = 0;
        glowHeight = 0;
    }

    private static int linkProgram(String fragmentSource) throws ReflectiveOperationException {
        int vertex = Gl.compileShader(Gl.VERTEX_SHADER, VERTEX_SHADER);
        int fragment;
        try {
            fragment = Gl.compileShader(Gl.FRAGMENT_SHADER, fragmentSource);
        } catch (ReflectiveOperationException | RuntimeException error) {
            Gl.deleteShader(vertex);
            throw error;
        }
        try {
            int linked = Gl.createProgram();
            Gl.attachShader(linked, vertex);
            Gl.attachShader(linked, fragment);
            Gl.linkProgram(linked);
            if (Gl.programStatus(linked) == 0) {
                throw new IllegalStateException("link failed: " + Gl.programLog(linked));
            }
            return linked;
        } finally {
            Gl.deleteShader(vertex);
            Gl.deleteShader(fragment);
        }
    }

     










    private static float echoDecay(long now, boolean echo) {
        long previous = lastFrameNanos;
        lastFrameNanos = now;
        if (!echo || !echoPrimed || previous == 0L) {
            return 0.0F;
        }
        double step = Math.min((now - previous) / 1.0e9D, MAX_ECHO_STEP);
        if (step <= 0.0D) {
            return 1.0F;
        }
        return (float) Math.pow(0.5D, step / ECHO_HALF_LIFE);
    }

     
    private static float finite(float value, float min, float max, float fallback) {
        if (!Float.isFinite(value)) {
            return fallback;
        }
        return Math.max(min, Math.min(max, value));
    }

    private static Throwable root(Throwable error) {
        Throwable cause = error;
        while (cause.getCause() != null && cause.getCause() != cause) {
            cause = cause.getCause();
        }
        return cause;
    }
    private static Method method(Class<?> owner, String name, Class<?>... parameters)
            throws NoSuchMethodException {
        Method found = owner.getMethod(name, parameters);
        found.setAccessible(true);
        return found;
    }

     





    private static final class Gl {
        static final int FRAMEBUFFER = 0x8D40;
        static final int DRAW_FRAMEBUFFER = 0x8CA9;
        static final int READ_FRAMEBUFFER = 0x8CA8;
        static final int DRAW_FRAMEBUFFER_BINDING = 0x8CA6;
        static final int READ_FRAMEBUFFER_BINDING = 0x8CAA;
        static final int FRAMEBUFFER_COMPLETE = 0x8CD5;
        static final int COLOR_ATTACHMENT0 = 0x8CE0;
        static final int DEPTH_ATTACHMENT = 0x8D00;
        static final int TEXTURE_2D = 0x0DE1;
        static final int DEPTH_COMPONENT = 0x1902;
        static final int DEPTH_COMPONENT24 = 0x81A6;
        static final int UNSIGNED_INT = 0x1405;
        static final int RGBA = 0x1908;
        static final int RGBA8 = 0x8058;
        static final int UNSIGNED_BYTE = 0x1401;
        static final int TEXTURE_MIN_FILTER = 0x2801;
        static final int TEXTURE_MAG_FILTER = 0x2800;
        static final int NEAREST = 0x2600;
        static final int TEXTURE_WRAP_S = 0x2802;
        static final int TEXTURE_WRAP_T = 0x2803;
        static final int CLAMP_TO_EDGE = 0x812F;
        static final int DEPTH_TEST = 0x0B71;
        static final int DEPTH_WRITEMASK = 0x0B72;
        static final int DEPTH_FUNC = 0x0B74;
        static final int LEQUAL = 0x0203;
        static final int BLEND = 0x0BE2;
        static final int BLEND_SRC_RGB = 0x80C9;
        static final int BLEND_DST_RGB = 0x80C8;
        static final int BLEND_SRC_ALPHA = 0x80CB;
        static final int BLEND_DST_ALPHA = 0x80CA;
        static final int BLEND_EQUATION_RGB = 0x8009;
        static final int CULL_FACE = 0x0B44;
        static final int SCISSOR_TEST = 0x0C11;
        static final int SCISSOR_BOX = 0x0C10;
        static final int VIEWPORT = 0x0BA2;
        static final int CURRENT_PROGRAM = 0x8B8D;
        static final int VERTEX_ARRAY_BINDING = 0x85B5;
        static final int TEXTURE0 = 0x84C0;
        static final int TEXTURE1 = 0x84C1;
        static final int TEXTURE_BINDING_2D = 0x8069;
        static final int LINEAR = 0x2601;
        static final int ZERO = 0;
        static final int DEPTH_BUFFER_BIT = 0x0100;
        static final int SRC_ALPHA = 0x0302;
        static final int ONE = 1;
        static final int FUNC_ADD = 0x8006;
        static final int VERTEX_SHADER = 0x8B31;
        static final int FRAGMENT_SHADER = 0x8B30;
        static final int COMPILE_STATUS = 0x8B81;
        static final int LINK_STATUS = 0x8B82;
        static final int TRIANGLE_STRIP = 0x0005;

        static boolean loaded;
        static boolean failed;

        static Method isEnabled;
        static Method getIntegerv;
        static Method drawArrays;
        static Method uniform1i;
        static Method uniform1f;
        static Method uniform2f;
        static Method uniform4f;

        static Method enableDepth;
        static Method disableDepth;
        static Method depthFunc;
        static Method depthMask;
        static Method enableBlend;
        static Method disableBlend;
        static Method blendFuncSeparate;
        static Method blendEquation;
        static Method enableCull;
        static Method disableCull;
        static Method enableScissor;
        static Method disableScissor;
        static Method scissorBox;
        static Method viewport;
        static Method colorMask;
        static Method clear;
        static Method clearDepth;
        static Method activeTexture;
        static Method getActiveTexture;
        static Method bindTexture;
        static Method useProgram;
        static Method genVertexArray;
        static Method bindVertexArray;
        static Method deleteVertexArray;
        static Method getInteger;
        static Method bindFramebuffer;
        static Method blitFramebuffer;
        static Method genFramebuffer;
        static Method checkFramebuffer;
        static Method framebufferTexture;
        static Method deleteFramebuffer;
        static Method genTexture;
        static Method deleteTexture;
        static Method texImage2D;
        static Method texParameter;
        static Method createShader;
        static Method shaderSource;
        static Method compileShader;
        static Method getShaderi;
        static Method getShaderLog;
        static Method deleteShader;
        static Method createProgram;
        static Method attachShader;
        static Method linkProgram;
        static Method getProgrami;
        static Method getProgramLog;
        static Method deleteProgram;
        static Method getUniformLocation;

        static Class<?> renderSystem;
        static Class<?> shaderProgram;
        static Method getShader;
        static Method setShader;
        static Method clearShader;
        static Method bindShader;

        static IntBuffer ints;

        private Gl() {}

         




        private static Class<?> shaderProgramClass(ClassLoader loader)
                throws ClassNotFoundException {
            try {
                return Class.forName("net.minecraft.class_5944", true, loader);
            } catch (ClassNotFoundException intermediary) {
                return Class.forName("net.minecraft.client.gl.ShaderProgram", true, loader);
            }
        }

        static synchronized void load(ClassLoader loader) {
            if (loaded || failed) {
                return;
            }
            try {
                Class<?> state =
                        Class.forName("com.mojang.blaze3d.platform.GlStateManager", true, loader);
                Class<?> gl11 = Class.forName("org.lwjgl.opengl.GL11", true, loader);
                Class<?> gl20 = Class.forName("org.lwjgl.opengl.GL20", true, loader);
                renderSystem = Class.forName("com.mojang.blaze3d.systems.RenderSystem", true, loader);
                shaderProgram = shaderProgramClass(loader);
                getShader = renderSystem.getMethod("getShader");
                setShader = renderSystem.getMethod("setShader", shaderProgram);
                clearShader = renderSystem.getMethod("clearShader");
                 
                 
                try {
                    bindShader = shaderProgram.getMethod("bind");
                } catch (NoSuchMethodException named) {
                    bindShader = shaderProgram.getMethod("method_34586");
                }

                isEnabled = gl11.getMethod("glIsEnabled", int.class);
                getIntegerv = gl11.getMethod("glGetIntegerv", int.class, IntBuffer.class);
                drawArrays = gl11.getMethod("glDrawArrays", int.class, int.class, int.class);
                uniform1i = gl20.getMethod("glUniform1i", int.class, int.class);
                uniform1f = gl20.getMethod("glUniform1f", int.class, float.class);
                uniform2f = gl20.getMethod("glUniform2f", int.class, float.class, float.class);
                uniform4f =
                        gl20.getMethod(
                                "glUniform4f", int.class, float.class, float.class, float.class, float.class);

                enableDepth = state.getMethod("_enableDepthTest");
                disableDepth = state.getMethod("_disableDepthTest");
                depthFunc = state.getMethod("_depthFunc", int.class);
                depthMask = state.getMethod("_depthMask", boolean.class);
                enableBlend = state.getMethod("_enableBlend");
                disableBlend = state.getMethod("_disableBlend");
                blendFuncSeparate =
                        state.getMethod("_blendFuncSeparate", int.class, int.class, int.class, int.class);
                blendEquation = state.getMethod("_blendEquation", int.class);
                enableCull = state.getMethod("_enableCull");
                disableCull = state.getMethod("_disableCull");
                enableScissor = state.getMethod("_enableScissorTest");
                disableScissor = state.getMethod("_disableScissorTest");
                scissorBox =
                        state.getMethod("_scissorBox", int.class, int.class, int.class, int.class);
                viewport = state.getMethod("_viewport", int.class, int.class, int.class, int.class);
                colorMask =
                        state.getMethod(
                                "_colorMask", boolean.class, boolean.class, boolean.class, boolean.class);
                clear = state.getMethod("_clear", int.class);
                clearDepth = state.getMethod("_clearDepth", double.class);
                activeTexture = state.getMethod("_activeTexture", int.class);
                getActiveTexture = state.getMethod("_getActiveTexture");
                bindTexture = state.getMethod("_bindTexture", int.class);
                useProgram = state.getMethod("_glUseProgram", int.class);
                genVertexArray = state.getMethod("_glGenVertexArrays");
                bindVertexArray = state.getMethod("_glBindVertexArray", int.class);
                deleteVertexArray = state.getMethod("_glDeleteVertexArrays", int.class);
                getInteger = state.getMethod("_getInteger", int.class);
                bindFramebuffer = state.getMethod("_glBindFramebuffer", int.class, int.class);
                blitFramebuffer =
                        state.getMethod(
                                "_glBlitFrameBuffer",
                                int.class,
                                int.class,
                                int.class,
                                int.class,
                                int.class,
                                int.class,
                                int.class,
                                int.class,
                                int.class,
                                int.class);
                genFramebuffer = state.getMethod("glGenFramebuffers");
                checkFramebuffer = state.getMethod("glCheckFramebufferStatus", int.class);
                framebufferTexture =
                        state.getMethod(
                                "_glFramebufferTexture2D", int.class, int.class, int.class, int.class, int.class);
                deleteFramebuffer = state.getMethod("_glDeleteFramebuffers", int.class);
                genTexture = state.getMethod("_genTexture");
                deleteTexture = state.getMethod("_deleteTexture", int.class);
                texImage2D =
                        state.getMethod(
                                "_texImage2D",
                                int.class,
                                int.class,
                                int.class,
                                int.class,
                                int.class,
                                int.class,
                                int.class,
                                int.class,
                                IntBuffer.class);
                texParameter = state.getMethod("_texParameter", int.class, int.class, int.class);
                createShader = state.getMethod("glCreateShader", int.class);
                shaderSource = state.getMethod("glShaderSource", int.class, String.class);
                compileShader = state.getMethod("glCompileShader", int.class);
                getShaderi = state.getMethod("glGetShaderi", int.class, int.class);
                getShaderLog = state.getMethod("glGetShaderInfoLog", int.class, int.class);
                deleteShader = state.getMethod("glDeleteShader", int.class);
                createProgram = state.getMethod("glCreateProgram");
                attachShader = state.getMethod("glAttachShader", int.class, int.class);
                linkProgram = state.getMethod("glLinkProgram", int.class);
                getProgrami = state.getMethod("glGetProgrami", int.class, int.class);
                getProgramLog = state.getMethod("glGetProgramInfoLog", int.class, int.class);
                deleteProgram = state.getMethod("glDeleteProgram", int.class);
                getUniformLocation =
                        state.getMethod("_glGetUniformLocation", int.class, CharSequence.class);

                ints = ByteBuffer.allocateDirect(64).order(ByteOrder.nativeOrder()).asIntBuffer();
                loaded = true;
            } catch (ReflectiveOperationException | RuntimeException error) {
                failed = true;
                throw new IllegalStateException("GL entry points unavailable: " + error, error);
            }
        }

         
        static void beginGlow(int target) throws ReflectiveOperationException {
            bindFramebuffer(FRAMEBUFFER, target);
            call(viewport, 0, 0, glowWidth, glowHeight);
            call(disableScissor);
            call(disableDepth);
            call(disableBlend);
            call(disableCull);
            call(colorMask, true, true, true, true);
        }

         








        static boolean copyDepth(State state, int target) throws ReflectiveOperationException {
            if (target == 0 || state.drawFramebuffer == target) {
                return false;
            }
            int sourceWidth = Math.min(width, state.viewportBox[2]);
            int sourceHeight = Math.min(height, state.viewportBox[3]);
            if (sourceWidth <= 0 || sourceHeight <= 0) {
                return false;
            }
            int x = state.viewportBox[0];
            int y = state.viewportBox[1];
            call(disableScissor);
            bindFramebuffer(READ_FRAMEBUFFER, state.drawFramebuffer);
            bindFramebuffer(DRAW_FRAMEBUFFER, target);
            call(
                    blitFramebuffer,
                    x,
                    y,
                    x + sourceWidth,
                    y + sourceHeight,
                    0,
                    0,
                    sourceWidth,
                    sourceHeight,
                    DEPTH_BUFFER_BIT,
                    NEAREST);
            bindFramebuffer(DRAW_FRAMEBUFFER, state.drawFramebuffer);
            bindFramebuffer(READ_FRAMEBUFFER, state.readFramebuffer);
            return true;
        }

         
        static void endGlow(State state) throws ReflectiveOperationException {
            bindFramebuffer(DRAW_FRAMEBUFFER, state.drawFramebuffer);
            bindFramebuffer(READ_FRAMEBUFFER, state.readFramebuffer);
            call(
                    viewport,
                    state.viewportBox[0],
                    state.viewportBox[1],
                    state.viewportBox[2],
                    state.viewportBox[3]);
        }

         
        static void extract(
                int mask,
                int shader,
                int maskLocation,
                int texelLocation,
                int radiusLocation,
                int vao,
                int targetWidth,
                int targetHeight,
                float radius)
                throws ReflectiveOperationException {
            call(activeTexture, TEXTURE0);
            call(bindTexture, mask);
            call(useProgram, shader);
            call(uniform1i, maskLocation, 0);
            call(uniform2f, texelLocation, 1.0F / targetWidth, 1.0F / targetHeight);
            call(uniform1f, radiusLocation, radius);
            call(bindVertexArray, vao);
            call(drawArrays, TRIANGLE_STRIP, 0, 4);
        }

         
        static void blur(
                int target,
                int source,
                int shader,
                int sourceLocation,
                int directionLocation,
                int sigmaLocation,
                int vao,
                int targetWidth,
                int targetHeight,
                float sigma,
                boolean horizontal)
                throws ReflectiveOperationException {
            bindFramebuffer(FRAMEBUFFER, target);
            call(viewport, 0, 0, targetWidth, targetHeight);
            call(activeTexture, TEXTURE0);
            call(bindTexture, source);
            call(useProgram, shader);
            call(uniform1i, sourceLocation, 0);
            call(
                    uniform2f,
                    directionLocation,
                    horizontal ? 1.0F / targetWidth : 0.0F,
                    horizontal ? 0.0F : 1.0F / targetHeight);
            call(uniform1f, sigmaLocation, sigma);
            call(bindVertexArray, vao);
            call(drawArrays, TRIANGLE_STRIP, 0, 4);
        }

         






        static void accumulate(
                int current,
                int echo,
                int shader,
                int currentLocation,
                int echoLocation,
                int decayLocation,
                int vao,
                float decay)
                throws ReflectiveOperationException {
            call(activeTexture, TEXTURE0);
            call(bindTexture, current);
            call(activeTexture, TEXTURE1);
            call(bindTexture, echo);
            call(useProgram, shader);
            call(uniform1i, currentLocation, 0);
            call(uniform1i, echoLocation, 1);
            call(uniform1f, decayLocation, decay);
            call(bindVertexArray, vao);
            call(drawArrays, TRIANGLE_STRIP, 0, 4);
        }

         










        static void composite(
                int mask,
                int glow,
                int shader,
                int maskLocation,
                int glowLocation,
                int colorLocation,
                int vao,
                float red,
                float green,
                float blue,
                float alpha)
                throws ReflectiveOperationException {
            call(disableDepth);
            call(depthMask, false);
            call(disableCull);
            call(disableScissor);
            call(enableBlend);
             
            call(blendFuncSeparate, ONE, ONE, ZERO, ONE);
            call(blendEquation, FUNC_ADD);
            call(activeTexture, TEXTURE0);
            call(bindTexture, mask);
            call(activeTexture, TEXTURE1);
            call(bindTexture, glow);
            call(useProgram, shader);
            call(uniform1i, maskLocation, 0);
            call(uniform1i, glowLocation, 1);
            call(uniform4f, colorLocation, red, green, blue, alpha);
            call(bindVertexArray, vao);
            call(drawArrays, TRIANGLE_STRIP, 0, 4);
        }

        static void requireComplete() throws ReflectiveOperationException {
            int status = checkFramebuffer();
            if (status != FRAMEBUFFER_COMPLETE) {
                throw new IllegalStateException(
                        "incomplete framebuffer: 0x" + Integer.toHexString(status));
            }
        }

        static void release(int[] programs, int vao, int[] framebuffers, int[] textures) {
            for (int program : programs) {
                if (program != 0) {
                    callQuietly(deleteProgram, program);
                }
            }
            if (vao != 0) {
                callQuietly(deleteVertexArray, vao);
            }
            for (int framebuffer : framebuffers) {
                if (framebuffer != 0) {
                    callQuietly(deleteFramebuffer, framebuffer);
                }
            }
            for (int texture : textures) {
                if (texture != 0) {
                    callQuietly(deleteTexture, texture);
                }
            }
        }

        static int genVertexArray() throws ReflectiveOperationException {
            return (Integer) call(genVertexArray);
        }

        static int genTexture() throws ReflectiveOperationException {
            return (Integer) call(genTexture);
        }

        static int genFramebuffer() throws ReflectiveOperationException {
            return (Integer) call(genFramebuffer);
        }

        static void bindTexture(int texture) throws ReflectiveOperationException {
            call(bindTexture, texture);
        }

        static void bindFramebuffer(int target, int framebuffer) throws ReflectiveOperationException {
            call(bindFramebuffer, target, framebuffer);
        }

        static void framebufferTexture(int attachment, int texture) throws ReflectiveOperationException {
            call(framebufferTexture, FRAMEBUFFER, attachment, TEXTURE_2D, texture, 0);
        }

        static int checkFramebuffer() throws ReflectiveOperationException {
            return (Integer) call(checkFramebuffer, FRAMEBUFFER);
        }

        static void texImage2D(int textureWidth, int textureHeight, boolean depth)
                throws ReflectiveOperationException {
            if (depth) {
                call(
                        texImage2D,
                        TEXTURE_2D,
                        0,
                        DEPTH_COMPONENT24,
                        textureWidth,
                        textureHeight,
                        0,
                        DEPTH_COMPONENT,
                        UNSIGNED_INT,
                        null);
            } else {
                call(
                        texImage2D,
                        TEXTURE_2D,
                        0,
                        RGBA8,
                        textureWidth,
                        textureHeight,
                        0,
                        RGBA,
                        UNSIGNED_BYTE,
                        null);
            }
        }

        static void textureParameters(int filter) throws ReflectiveOperationException {
            call(texParameter, TEXTURE_2D, TEXTURE_MIN_FILTER, filter);
            call(texParameter, TEXTURE_2D, TEXTURE_MAG_FILTER, filter);
            call(texParameter, TEXTURE_2D, TEXTURE_WRAP_S, CLAMP_TO_EDGE);
            call(texParameter, TEXTURE_2D, TEXTURE_WRAP_T, CLAMP_TO_EDGE);
        }

        static int compileShader(int type, String source) throws ReflectiveOperationException {
            int shader = (Integer) call(createShader, type);
            call(shaderSource, shader, source);
            call(compileShader, shader);
            if ((Integer) call(getShaderi, shader, COMPILE_STATUS) == 0) {
                throw new IllegalStateException("compile failed: " + call(getShaderLog, shader, 8192));
            }
            return shader;
        }

        static int createProgram() throws ReflectiveOperationException {
            return (Integer) call(createProgram);
        }

        static void attachShader(int program, int shader) throws ReflectiveOperationException {
            call(attachShader, program, shader);
        }

        static void linkProgram(int program) throws ReflectiveOperationException {
            call(linkProgram, program);
        }

        static int programStatus(int program) throws ReflectiveOperationException {
            return (Integer) call(getProgrami, program, LINK_STATUS);
        }

        static String programLog(int program) throws ReflectiveOperationException {
            return (String) call(getProgramLog, program, 8192);
        }

        static void deleteShader(int shader) {
            callQuietly(deleteShader, shader);
        }

        static int uniform(int program, String name) throws ReflectiveOperationException {
            return (Integer) call(getUniformLocation, program, name);
        }

        static int getInt(int pname) throws ReflectiveOperationException {
            return (Integer) call(getInteger, pname);
        }

        static int[] query(int pname, int count) throws ReflectiveOperationException {
            ints.clear();
            call(getIntegerv, pname, ints);
            int[] values = new int[count];
            for (int i = 0; i < count; i++) {
                values[i] = ints.get(i);
            }
            return values;
        }

        static boolean enabled(int capability) throws ReflectiveOperationException {
            return (Boolean) call(isEnabled, capability);
        }

        static Object call(Method method, Object... arguments) throws ReflectiveOperationException {
            try {
                return method.invoke(null, arguments);
            } catch (java.lang.reflect.InvocationTargetException error) {
                Throwable cause = error.getCause();
                if (cause instanceof RuntimeException runtime) {
                    throw runtime;
                }
                if (cause instanceof Error fatal) {
                    throw fatal;
                }
                throw error;
            }
        }

        static void callQuietly(Method method, Object... arguments) {
            try {
                call(method, arguments);
            } catch (ReflectiveOperationException | RuntimeException error) {
                System.err.println("Hand Aura: GL cleanup: " + error);
            }
        }

         
        static final class State {
            final boolean depth;
            final boolean blend;
            final boolean cull;
            final boolean scissor;
            final boolean depthWrite;
            final int depthFuncValue;
            final int sourceRgb;
            final int destinationRgb;
            final int sourceAlpha;
            final int destinationAlpha;
            final int equation;
            final int[] scissorRect;
            final int[] viewportBox;
            final int drawFramebuffer;
            final int readFramebuffer;
            final int shader;
            final int vertexArray;
            final int activeUnit;
            final int texture;
            final int texture1;
            final Object gameShader;

            private State(
                    boolean depth,
                    boolean blend,
                    boolean cull,
                    boolean scissor,
                    boolean depthWrite,
                    int depthFuncValue,
                    int sourceRgb,
                    int destinationRgb,
                    int sourceAlpha,
                    int destinationAlpha,
                    int equation,
                    int[] scissorRect,
                    int[] viewportBox,
                    int drawFramebuffer,
                    int readFramebuffer,
                    int shader,
                    int vertexArray,
                    int activeUnit,
                    int texture,
                    int texture1,
                    Object gameShader) {
                this.depth = depth;
                this.blend = blend;
                this.cull = cull;
                this.scissor = scissor;
                this.depthWrite = depthWrite;
                this.depthFuncValue = depthFuncValue;
                this.sourceRgb = sourceRgb;
                this.destinationRgb = destinationRgb;
                this.sourceAlpha = sourceAlpha;
                this.destinationAlpha = destinationAlpha;
                this.equation = equation;
                this.scissorRect = scissorRect;
                this.viewportBox = viewportBox;
                this.drawFramebuffer = drawFramebuffer;
                this.readFramebuffer = readFramebuffer;
                this.shader = shader;
                this.vertexArray = vertexArray;
                this.activeUnit = activeUnit;
                this.texture = texture;
                this.texture1 = texture1;
                this.gameShader = gameShader;
            }

            static State capture(ClassLoader loader) throws ReflectiveOperationException {
                int active = (Integer) call(getActiveTexture);
                call(activeTexture, TEXTURE0);
                int texture = (Integer) call(getInteger, TEXTURE_BINDING_2D);
                call(activeTexture, TEXTURE1);
                int texture1 = (Integer) call(getInteger, TEXTURE_BINDING_2D);
                call(activeTexture, active);
                return new State(
                        enabled(DEPTH_TEST),
                        enabled(BLEND),
                        enabled(CULL_FACE),
                        enabled(SCISSOR_TEST),
                        (Integer) call(getInteger, DEPTH_WRITEMASK) != 0,
                        (Integer) call(getInteger, DEPTH_FUNC),
                        (Integer) call(getInteger, BLEND_SRC_RGB),
                        (Integer) call(getInteger, BLEND_DST_RGB),
                        (Integer) call(getInteger, BLEND_SRC_ALPHA),
                        (Integer) call(getInteger, BLEND_DST_ALPHA),
                        (Integer) call(getInteger, BLEND_EQUATION_RGB),
                        query(SCISSOR_BOX, 4),
                        query(VIEWPORT, 4),
                        (Integer) call(getInteger, DRAW_FRAMEBUFFER_BINDING),
                        (Integer) call(getInteger, READ_FRAMEBUFFER_BINDING),
                        (Integer) call(getInteger, CURRENT_PROGRAM),
                        (Integer) call(getInteger, VERTEX_ARRAY_BINDING),
                        active,
                        texture,
                        texture1,
                        getShader.invoke(null));
            }

            void restore(ClassLoader loader) throws ReflectiveOperationException {
                call(activeTexture, TEXTURE0);
                call(bindTexture, texture);
                call(activeTexture, TEXTURE1);
                call(bindTexture, texture1);
                call(activeTexture, activeUnit);
                call(bindVertexArray, vertexArray);
                call(useProgram, shader);
                if (blend) {
                    call(enableBlend);
                    call(blendFuncSeparate, sourceRgb, destinationRgb, sourceAlpha, destinationAlpha);
                    call(blendEquation, equation);
                } else {
                    call(disableBlend);
                }
                if (depth) {
                    call(enableDepth);
                    call(depthFunc, depthFuncValue);
                } else {
                    call(disableDepth);
                }
                call(depthMask, depthWrite);
                if (cull) {
                    call(enableCull);
                } else {
                    call(disableCull);
                }
                if (scissor) {
                    call(enableScissor);
                    call(scissorBox, scissorRect[0], scissorRect[1], scissorRect[2], scissorRect[3]);
                } else {
                    call(disableScissor);
                }
                call(viewport, viewportBox[0], viewportBox[1], viewportBox[2], viewportBox[3]);
                call(bindFramebuffer, DRAW_FRAMEBUFFER, drawFramebuffer);
                call(bindFramebuffer, READ_FRAMEBUFFER, readFramebuffer);
                call(colorMask, true, true, true, true);
                 
                 
                if (gameShader != null) {
                    setShader.invoke(null, gameShader);
                    bindShader.invoke(gameShader);
                } else {
                    clearShader.invoke(null);
                    call(useProgram, shader);
                }
            }
        }
    }
}
