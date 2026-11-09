package infered.infered.client;

import infered.infered.Infered;
import infered.infered.client.menu.ModMenu;
import infered.infered.client.modules.ModuleManager;
import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientChunkEvents;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.event.player.UseBlockCallback;
import net.fabricmc.fabric.api.client.keybinding.v1.KeyBindingHelper;
import net.fabricmc.fabric.api.client.networking.v1.ClientPlayConnectionEvents;
import net.fabricmc.fabric.api.client.rendering.v1.HudRenderCallback;
import net.fabricmc.fabric.api.client.rendering.v1.WorldRenderContext;
import net.fabricmc.fabric.api.client.rendering.v1.WorldRenderEvents;
import net.fabricmc.fabric.api.client.screen.v1.ScreenEvents;
import net.minecraft.client.MinecraftClient;
import net.minecraft.client.gui.DrawContext;
import net.minecraft.client.option.KeyBinding;
import net.minecraft.client.util.InputUtil;
import org.lwjgl.glfw.GLFW;

public class InferedClient implements ClientModInitializer {
    public static KeyBinding MENU_KEY;
    private static boolean prevKeyDown = false;

    private static final int RED = 0xFFE02020;
    private static final int BLACK = 0xFF000000;

    @Override
    public void onInitializeClient() {
        InferedClientHolder.init();
        Infered.LOGGER.info("Infered client initialized! Modules: FPS, Fullbright, Watermark, AutoTotem, AutoArmor, FreeCam, Criticals, MobAura, Velocity.");

        MENU_KEY = KeyBindingHelper.registerKeyBinding(new KeyBinding(
            "key.infered.menu",
            InputUtil.Type.KEYSYM,
            GLFW.GLFW_KEY_RIGHT_SHIFT,
            "category.infered"
        ));

         
         
        ClientTickEvents.END_CLIENT_TICK.register(client -> {
            InputUtil.Key bound = KeyBindingHelper.getBoundKeyOf(MENU_KEY);
            boolean down = bound.getCategory() == InputUtil.Type.KEYSYM
                && bound.getCode() != GLFW.GLFW_KEY_UNKNOWN
                && InputUtil.isKeyPressed(client.getWindow().getHandle(), bound.getCode());

            if (down && !prevKeyDown) {
                 
                boolean typing = client.currentScreen != null
                    && client.currentScreen.getFocused() instanceof net.minecraft.client.gui.widget.TextFieldWidget;
                if (!typing) {
                    if (client.currentScreen instanceof ModMenu menu) {
                        menu.close();
                    } else {
                         
                        client.setScreen(new ModMenu());
                    }
                }
            }
            prevKeyDown = down;
        });

         
         
         
        ClientTickEvents.END_CLIENT_TICK.register(client -> ModuleManager.get().tick(client));

         
         
        UseBlockCallback.EVENT.register((player, world, hand, hitResult) ->
            ModuleManager.get().testModule676767.onUseBlock(player, world, hand, hitResult));

         
         
         
        ClientChunkEvents.CHUNK_LOAD.register((world, chunk) -> ModuleManager.get().onChunkLoad(chunk));
        ClientChunkEvents.CHUNK_UNLOAD.register((world, chunk) -> ModuleManager.get().onChunkUnload(chunk));
        ClientPlayConnectionEvents.JOIN.register((handler, sender, client) ->
            client.execute(() -> {
                ModuleManager.get().blockEsp.rescanAll();
                ModuleManager.get().storageEsp.rescanAll();
            }));
        ClientPlayConnectionEvents.DISCONNECT.register((handler, client) -> {
            ModuleManager.get().blockEsp.clear();
            ModuleManager.get().storageEsp.clear();
            ModuleManager.get().stashFinder.saveJson();
            ModuleManager.get().stashFinder.saveCsv();
        });

         
        HudRenderCallback.EVENT.register((context, tickCounter) -> ModuleManager.get().renderHud(context));

         
         
         
         
         
        WorldRenderEvents.LAST.register(InferedClient::renderWorldEsp);

         
        ScreenEvents.AFTER_INIT.register((client, screen, scaledWidth, scaledHeight) ->
            ScreenEvents.afterRender(screen).register((s, context, mouseX, mouseY, tickDelta) -> {
                if (client.world == null) {
                    drawMenuWatermark(context);
                }
            }));
    }

    private static void renderWorldEsp(WorldRenderContext context) {
        ModuleManager.get().renderLast(context);
    }

     
    private static void drawMenuWatermark(DrawContext context) {
        MinecraftClient client = MinecraftClient.getInstance();
        context.drawText(client.textRenderer, "Infered", 4, 4, RED, true);
        int w = client.textRenderer.getWidth("Infered");
        context.drawText(client.textRenderer, " by thefinalblow", 4 + w, 4, BLACK, true);
    }
}
