package infered.infered.client.render;

import net.fabricmc.fabric.api.client.rendering.v1.WorldRenderContext;
import net.minecraft.client.render.BufferBuilder;
import net.minecraft.client.render.BufferRenderer;
import net.minecraft.client.render.Camera;
import net.minecraft.client.gl.ShaderProgramKeys;
import net.minecraft.client.render.Tessellator;
import net.minecraft.client.render.VertexFormat;
import net.minecraft.client.render.VertexFormats;
import net.minecraft.client.util.math.MatrixStack;
import net.minecraft.util.math.BlockPos;
import net.minecraft.util.math.Box;
import net.minecraft.util.math.Vec3d;
import com.mojang.blaze3d.systems.RenderSystem;
import org.joml.Matrix4fStack;

 















public final class InferedDrawer {
     
    private static final float FILL_ALPHA = 0.25f;

    private InferedDrawer() {}

     

     
    public static void begin(WorldRenderContext ctx) {
        Matrix4fStack stack = RenderSystem.getModelViewStack();
         
         
         
         
         
         
         
        stack.pushMatrix();
        RenderSystem.enableBlend();
        RenderSystem.defaultBlendFunc();
        RenderSystem.disableDepthTest();
        RenderSystem.disableCull();
        RenderSystem.setShader(ShaderProgramKeys.POSITION_COLOR);
    }

    public static void end(WorldRenderContext ctx) {
        Matrix4fStack stack = RenderSystem.getModelViewStack();
        stack.popMatrix();
        RenderSystem.enableDepthTest();
        RenderSystem.enableCull();
        RenderSystem.disableBlend();
    }

     

     
    public static void drawLine(WorldRenderContext ctx, Vec3d from, Vec3d to, int argb) {
        Camera cam = ctx.camera();
        float[] c = rgba(argb);
        Tessellator tess = Tessellator.getInstance();
        BufferBuilder buf = tess.begin(VertexFormat.DrawMode.DEBUG_LINES, VertexFormats.POSITION_COLOR);
        vertex(buf, cam, from.x, from.y, from.z, c);
        vertex(buf, cam, to.x, to.y, to.z, c);
        BufferRenderer.drawWithGlobalProgram(buf.end());
    }

     
    public static void drawTracer(WorldRenderContext ctx, Vec3d target, int argb) {
         
         
        drawLine(ctx, ctx.camera().getPos(), target, argb);
    }

     
    public static void drawBox(WorldRenderContext ctx, Box box, int argb) {
        Camera cam = ctx.camera();
        float[] c = rgba(argb);
        double x0 = box.minX - cam.getPos().x, y0 = box.minY - cam.getPos().y, z0 = box.minZ - cam.getPos().z;
        double x1 = box.maxX - cam.getPos().x, y1 = box.maxY - cam.getPos().y, z1 = box.maxZ - cam.getPos().z;

        Tessellator tess = Tessellator.getInstance();
        BufferBuilder buf = tess.begin(VertexFormat.DrawMode.DEBUG_LINES, VertexFormats.POSITION_COLOR);

         
        edge(buf, cam, x0, y0, z0, x1, y0, z0, c);
        edge(buf, cam, x1, y0, z0, x1, y0, z1, c);
        edge(buf, cam, x1, y0, z1, x0, y0, z1, c);
        edge(buf, cam, x0, y0, z1, x0, y0, z0, c);
         
        edge(buf, cam, x0, y1, z0, x1, y1, z0, c);
        edge(buf, cam, x1, y1, z0, x1, y1, z1, c);
        edge(buf, cam, x1, y1, z1, x0, y1, z1, c);
        edge(buf, cam, x0, y1, z1, x0, y1, z0, c);
         
        edge(buf, cam, x0, y0, z0, x0, y1, z0, c);
        edge(buf, cam, x1, y0, z0, x1, y1, z0, c);
        edge(buf, cam, x1, y0, z1, x1, y1, z1, c);
        edge(buf, cam, x0, y0, z1, x0, y1, z1, c);

        BufferRenderer.drawWithGlobalProgram(buf.end());
    }

     
    public static void drawFilledBox(WorldRenderContext ctx, Box box, int argb) {
        Camera cam = ctx.camera();
        float[] c = rgba(argb);
        c = new float[]{c[0], c[1], c[2], FILL_ALPHA};
        double x0 = box.minX - cam.getPos().x, y0 = box.minY - cam.getPos().y, z0 = box.minZ - cam.getPos().z;
        double x1 = box.maxX - cam.getPos().x, y1 = box.maxY - cam.getPos().y, z1 = box.maxZ - cam.getPos().z;

        Tessellator tess = Tessellator.getInstance();
        BufferBuilder buf = tess.begin(VertexFormat.DrawMode.QUADS, VertexFormats.POSITION_COLOR);

        quad(buf, cam, x0, y0, z0, x1, y0, z0, x1, y1, z0, x0, y1, z0, c);  
        quad(buf, cam, x0, y0, z1, x1, y0, z1, x1, y1, z1, x0, y1, z1, c);  
        quad(buf, cam, x0, y0, z0, x1, y0, z0, x1, y0, z1, x0, y0, z1, c);  
        quad(buf, cam, x0, y1, z0, x1, y1, z0, x1, y1, z1, x0, y1, z1, c);  
        quad(buf, cam, x0, y0, z0, x0, y0, z1, x0, y1, z1, x0, y1, z0, c);  
        quad(buf, cam, x1, y0, z0, x1, y0, z1, x1, y1, z1, x1, y1, z0, c);  

        BufferRenderer.drawWithGlobalProgram(buf.end());
    }

     
    public static void drawQuad(WorldRenderContext ctx,
                                double x1, double y1, double z1,
                                double x2, double y2, double z2,
                                double x3, double y3, double z3,
                                double x4, double y4, double z4, int argb) {
        Camera cam = ctx.camera();
        float[] c = rgba(argb);
        Tessellator tess = Tessellator.getInstance();
        BufferBuilder buf = tess.begin(VertexFormat.DrawMode.QUADS, VertexFormats.POSITION_COLOR);
        vertex(buf, cam, x1, y1, z1, c);
        vertex(buf, cam, x2, y2, z2, c);
        vertex(buf, cam, x3, y3, z3, c);
        vertex(buf, cam, x4, y4, z4, c);
        BufferRenderer.drawWithGlobalProgram(buf.end());
    }

     

     



    public static void drawBlockRegion(WorldRenderContext ctx, BlockPos pos, int expand, int argb, boolean fill) {
        Box box = new Box(
            pos.getX() - expand, pos.getY() - expand, pos.getZ() - expand,
            pos.getX() + 1 + expand, pos.getY() + 1 + expand, pos.getZ() + 1 + expand);
        if (fill) {
            drawFilledBox(ctx, box, argb);
        }
        drawBox(ctx, box, argb);
    }

     
    public static void drawBlockTracer(WorldRenderContext ctx, BlockPos pos, int argb) {
        drawTracer(ctx, new Vec3d(pos.getX() + 0.5, pos.getY() + 0.5, pos.getZ() + 0.5), argb);
    }

     

    private static void edge(BufferBuilder buf, Camera cam,
                             double ax, double ay, double az,
                             double bx, double by, double bz, float[] c) {
        vertex(buf, cam, ax, ay, az, c);
        vertex(buf, cam, bx, by, bz, c);
    }

    private static void quad(BufferBuilder buf, Camera cam,
                             double ax, double ay, double az,
                             double bx, double by, double bz,
                             double cx, double cy, double cz,
                             double dx, double dy, double dz, float[] c) {
        vertex(buf, cam, ax, ay, az, c);
        vertex(buf, cam, bx, by, bz, c);
        vertex(buf, cam, cx, cy, cz, c);
        vertex(buf, cam, dx, dy, dz, c);
    }

    private static void vertex(BufferBuilder buf, Camera cam,
                               double x, double y, double z, float[] c) {
        buf.vertex((float) (x - cam.getPos().x),
                   (float) (y - cam.getPos().y),
                   (float) (z - cam.getPos().z))
           .color(c[0], c[1], c[2], c[3]);
    }

    private static float[] rgba(int argb) {
        return new float[]{
            (argb >> 16 & 0xFF) / 255f,
            (argb >> 8 & 0xFF) / 255f,
            (argb & 0xFF) / 255f,
            (argb >>> 24 & 0xFF) / 255f
        };
    }
}
