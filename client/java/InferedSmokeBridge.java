import java.io.ByteArrayInputStream;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.lang.reflect.Proxy;
import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.Map;
import java.util.function.Consumer;

 
public final class InferedSmokeBridge {
     










    private static final Map<String, Object> loaders = new HashMap<>();

     
    private static final String WORLD_PASS = "smoke";
    private static Object minecraft;
    private static Method matrixGet;

    private InferedSmokeBridge() {}

    public static volatile boolean ACTIVE;

    public static void afterWorldRender(Object renderer, Object camera, Object view, Object projection) {
        if (ACTIVE) drawWorldRender(renderer, camera, view, projection);
    }

    private static native void drawWorldRender(Object renderer, Object camera, Object view, Object projection);

    public static float[] matrix(Object matrix) throws Exception {
        if (matrixGet == null) {
            matrixGet = Class.forName("org.joml.Matrix4f").getMethod("get", float[].class);
        }
        return (float[]) matrixGet.invoke(matrix, (Object) new float[16]);
    }

     






    public static synchronized int loadShader(Object client, String name, String vertex, String fragment) throws Exception {
        if (loaders.containsKey(name)) throw new IllegalStateException(name + " shader already loaded");
        minecraft = client;
        ClassLoader cl = client.getClass().getClassLoader();
        Class<?> id = Class.forName("net.minecraft.class_2960", true, cl);
        Class<?> format = Class.forName("net.minecraft.class_293", true, cl);
        Class<?> defines = Class.forName("net.minecraft.class_10149", true, cl);
        Class<?> keyClass = Class.forName("net.minecraft.class_10156", true, cl);
        Class<?> loaderClass = Class.forName("net.minecraft.class_10151", true, cl);
        Class<?> textureManager = Class.forName("net.minecraft.class_1060", true, cl);
        Class<?> factory = Class.forName("net.minecraft.class_5912", true, cl);
        Class<?> resource = Class.forName("net.minecraft.class_3298", true, cl);
        Class<?> pack = Class.forName("net.minecraft.class_3262", true, cl);
        Class<?> supplier = Class.forName("net.minecraft.class_7367", true, cl);
        Object positionFormat = Class.forName("net.minecraft.class_290", true, cl).getField("field_1592").get(null);
        Object emptyDefines = defines.getConstructor(Map.class, java.util.Set.class)
                .newInstance(Map.of(), java.util.Set.of());
        Method identifier = id.getMethod("method_60655", String.class, String.class);
        Object keyId = identifier.invoke(null, "infered", "core/" + name);
        Object key = keyClass.getConstructor(id, format, defines).newInstance(keyId, positionFormat, emptyDefines);
        Map<Object, Object> resources = new HashMap<>();
        String config = "{\"vertex\":\"infered:core/" + name + "\",\"fragment\":\"infered:core/" + name + "\",\"samplers\":[],\"uniforms\":[]}";
        String[] paths = {"shaders/core/" + name + ".json", "shaders/core/" + name + ".vsh", "shaders/core/" + name + ".fsh"};
        String[] sources = {config, vertex, fragment};
        for (int i = 0; i < paths.length; i++) {
            byte[] bytes = sources[i].getBytes(StandardCharsets.UTF_8);
            Object input = Proxy.newProxyInstance(cl, new Class<?>[]{supplier}, (proxy, method, args) -> {
                if (method.getName().equals("get")) return new ByteArrayInputStream(bytes);
                if (method.getName().equals("toString")) return "InferedSmokeInput";
                if (method.getName().equals("hashCode")) return System.identityHashCode(proxy);
                if (method.getName().equals("equals")) return proxy == args[0];
                throw new UnsupportedOperationException(method.toString());
            });
            Object value = resource.getConstructor(pack, supplier).newInstance(null, input);
            resources.put(identifier.invoke(null, "infered", paths[i]), value);
        }
        Object resourceFactory = factory.getMethod("fromMap", Map.class).invoke(null, resources);
        Object textures = client.getClass().getMethod("method_1531").invoke(client);
        Consumer<Exception> onError = error -> { throw new IllegalStateException(name + " shader loader failed", error); };
        Object nextLoader = loaderClass.getConstructor(textureManager, Consumer.class).newInstance(textures, onError);
        try {
            Object keys = java.lang.reflect.Array.newInstance(keyClass, 1);
            java.lang.reflect.Array.set(keys, 0, key);
            loaderClass.getMethod("method_62944", factory, keys.getClass()).invoke(nextLoader, resourceFactory, keys);
            Object program = loaderClass.getMethod("method_64062", keyClass).invoke(nextLoader, key);
            int glRef = (int) program.getClass().getMethod("method_1270").invoke(program);
            loaders.put(name, nextLoader);
            return glRef;
        } catch (Exception e) {
            ((AutoCloseable) nextLoader).close();
            Throwable cause = e instanceof InvocationTargetException invocation ? invocation.getCause() : e;
            throw new IllegalStateException(name + " ShaderLoader.preload failed", cause);
        }
    }

     





    public static synchronized int reloadShader(Object client, String name, String vertex, String fragment) throws Exception {
        Object previous = loaders.remove(name);
        if (previous != null) {
            try {
                ((AutoCloseable) previous).close();
            } catch (Exception ignored) {
                 
            }
        }
        return loadShader(client, name, vertex, fragment);
    }

     




    public static synchronized void release(
            int vao, int vbo, int depth) throws Exception {
        Object oldLoader = loaders.remove(WORLD_PASS);
        if (minecraft == null) {
             
            return;
        }
        Object client = minecraft;
        Runnable cleanup = () -> {
            try {
                ClassLoader cl = client.getClass().getClassLoader();
                Class<?> gl30 = Class.forName("org.lwjgl.opengl.GL30", true, cl);
                Class<?> gl15 = Class.forName("org.lwjgl.opengl.GL15", true, cl);
                Class<?> gl11 = Class.forName("org.lwjgl.opengl.GL11", true, cl);
                if (vao != 0) gl30.getMethod("glDeleteVertexArrays", int.class).invoke(null, vao);
                if (vbo != 0) gl15.getMethod("glDeleteBuffers", int.class).invoke(null, vbo);
                if (depth != 0) gl11.getMethod("glDeleteTextures", int.class).invoke(null, depth);
                if (oldLoader != null) ((AutoCloseable) oldLoader).close();
            } catch (Exception e) {
                System.err.println("[Smoke] cleanup failed: " + e);
            }
        };
         
        ((java.util.concurrent.Executor) client).execute(cleanup);
    }
}
