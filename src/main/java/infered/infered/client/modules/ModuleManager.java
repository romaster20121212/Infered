package infered.infered.client.modules;

import infered.infered.client.InferedClientHolder;
import infered.infered.client.InferedConfig;
import infered.infered.client.render.InferedDrawer;
import net.fabricmc.fabric.api.client.rendering.v1.WorldRenderContext;
import net.minecraft.client.MinecraftClient;
import net.minecraft.client.gui.DrawContext;
import net.minecraft.util.math.BlockPos;
import net.minecraft.block.BlockState;
import net.minecraft.world.chunk.WorldChunk;

import java.util.ArrayList;
import java.util.List;
import java.util.Set;

 


public class ModuleManager {
    private static final ModuleManager INSTANCE = new ModuleManager();

    public final ShowFps showFps = new ShowFps();
    public final Fullbright fullbright = new Fullbright();
    public final Watermark watermark = new Watermark();
    public final BlockEsp blockEsp = new BlockEsp();
    public final StorageEsp storageEsp = new StorageEsp();
    public final EntityEsp entityEsp = new EntityEsp();
    public final StashFinder stashFinder = new StashFinder();
    public final AutoTotem autoTotem = new AutoTotem();
    public final AutoArmor autoArmor = new AutoArmor();
    public final FreeCam freeCam = new FreeCam();
    public final Criticals criticals = new Criticals();
    public final MobAura mobAura = new MobAura();
    public final Velocity velocity = new Velocity();
    public final CrystalFlowTest crystalFlowTest = new CrystalFlowTest();
    public final AnchorAuraTest anchorAuraTest = new AnchorAuraTest();
    public final TestModule676767 testModule676767 = new TestModule676767();

    private final List<Module> modules = List.of(
        showFps, fullbright, watermark, blockEsp, storageEsp, entityEsp, stashFinder,
        autoTotem, autoArmor, freeCam, criticals, mobAura, velocity,
        crystalFlowTest, anchorAuraTest, testModule676767);

    private final boolean[] lastApplied;

    public static ModuleManager get() {
        return INSTANCE;
    }

    private ModuleManager() {
        lastApplied = new boolean[modules.size()];
         
        for (Module m : modules) {
            if (m instanceof ChunkEsp esp) {
                Set<String> saved = InferedClientHolder.config.getBlockSelection(m.getId());
                if (!saved.isEmpty()) {
                    esp.selectedBlocks.clear();
                    esp.selectedBlocks.addAll(saved);
                }
            } else if (m instanceof StashFinder sf) {
                sf.loadSettings();
            } else if (m instanceof TestModule676767 crystalBuilder) {
                crystalBuilder.loadSettings();
            }
        }
    }

    public List<Module> getModules() {
        return modules;
    }

    public Module find(String id) {
        for (Module m : modules) {
            if (m.getId().equals(id)) return m;
        }
        return null;
    }

     
    public void tick(MinecraftClient client) {
        InferedConfig cfg = InferedClientHolder.config;
        for (int i = 0; i < modules.size(); i++) {
            Module m = modules.get(i);
            boolean shouldBeEnabled = cfg.isEnabled(m.getId());
            if (shouldBeEnabled != lastApplied[i]) {
                m.setEnabled(shouldBeEnabled);
                lastApplied[i] = shouldBeEnabled;
            }
            m.tick(client);
        }
    }

     
    public void onChunkLoad(WorldChunk chunk) {
        int bottomY = MinecraftClient.getInstance().world != null
            ? MinecraftClient.getInstance().world.getBottomY() : -64;
        blockEsp.onChunkLoad(chunk, bottomY);
        storageEsp.onChunkLoad(chunk, bottomY);
        stashFinder.onChunkLoad(chunk);
    }

     
    public void onChunkUnload(WorldChunk chunk) {
        blockEsp.onChunkUnload(chunk);
        storageEsp.onChunkUnload(chunk);
    }

     
    public void onBlockUpdate(BlockPos pos, BlockState oldState, BlockState newState) {
        blockEsp.onBlockUpdate(pos, oldState, newState);
        storageEsp.onBlockUpdate(pos, oldState, newState);
    }

    public void renderHud(DrawContext context) {
        for (Module m : modules) {
            m.renderHud(context);
        }
    }

     




    public void renderLast(WorldRenderContext ctx) {
        InferedDrawer.begin(ctx);
        try {
            for (Module m : modules) {
                m.renderLast(ctx);
            }
        } finally {
            InferedDrawer.end(ctx);
        }
    }
}
