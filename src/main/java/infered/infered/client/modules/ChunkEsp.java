package infered.infered.client.modules;

import infered.infered.client.InferedClientHolder;
import infered.infered.client.render.InferedDrawer;
import it.unimi.dsi.fastutil.longs.Long2ObjectMap;
import it.unimi.dsi.fastutil.longs.Long2ObjectOpenHashMap;
import net.fabricmc.fabric.api.client.rendering.v1.WorldRenderContext;
import net.minecraft.block.Block;
import net.minecraft.block.BlockState;
import net.minecraft.client.MinecraftClient;
import net.minecraft.registry.Registries;
import net.minecraft.util.math.BlockPos;
import net.minecraft.util.math.ChunkPos;
import net.minecraft.util.math.Vec3d;
import net.minecraft.world.chunk.ChunkSection;
import net.minecraft.world.chunk.WorldChunk;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

 










public abstract class ChunkEsp extends Module {
    protected static final ExecutorService WORKER = Executors.newSingleThreadExecutor();

     
    protected static final class EspBlock {
        public final int x, y, z;
        public final Block block;
        public Group group;

        public EspBlock(int x, int y, int z, Block block) {
            this.x = x; this.y = y; this.z = z;
            this.block = block;
        }

         
        public static long key(int x, int y, int z) {
            return ((long) (x & 0x3FFFFFF) << 38) | ((long) (z & 0x3FFFFFF) << 12) | (y & 0xFFF);
        }
        public long key() { return key(x, y, z); }
    }

     
    protected static final class Group {
        public final List<EspBlock> blocks = new ArrayList<>();
        public double sumX, sumY, sumZ;

        public void add(EspBlock b) {
            blocks.add(b);
            sumX += b.x; sumY += b.y; sumZ += b.z;
            b.group = this;
        }

        public Vec3d center() {
            double n = blocks.size();
            return new Vec3d(sumX / n + 0.5, sumY / n + 0.5, sumZ / n + 0.5);
        }
    }

     
    protected static final class Chunk {
        public final int x, z;
        public Long2ObjectMap<EspBlock> blocks;
        public volatile boolean dirty;

        public Chunk(int x, int z) { this.x = x; this.z = z; }

        public int size() { return blocks == null ? 0 : blocks.size(); }

        public boolean add(BlockPos pos, Block block) {
            if (blocks == null) blocks = new Long2ObjectOpenHashMap<>(64);
            long key = EspBlock.key(pos.getX(), pos.getY(), pos.getZ());
            if (blocks.containsKey(key)) return false;
            blocks.put(key, new EspBlock(pos.getX(), pos.getY(), pos.getZ(), block));
            dirty = true;
            return true;
        }

        public boolean remove(BlockPos pos) {
            if (blocks == null) return false;
            boolean removed = blocks.remove(EspBlock.key(pos.getX(), pos.getY(), pos.getZ())) != null;
            if (removed) dirty = true;
            return removed;
        }
    }

     

     
    public final Set<String> selectedBlocks = new HashSet<>();
     
    protected java.util.function.Predicate<Block> storageFilter = b -> true;

    protected final Long2ObjectMap<Chunk> chunks = new Long2ObjectOpenHashMap<>();
    protected final List<Group> groups = new ArrayList<>();
     
    private volatile Long2ObjectMap<EspBlock> renderBlocks = new Long2ObjectOpenHashMap<>();

    protected ChunkEsp(String id, String name, String description) {
        super(id, name, description, true);
    }

     

    public void addBlock(String blockId) {
        if (selectedBlocks.add(blockId)) {
            InferedClientHolder.config.setBlockSelection(id, selectedBlocks);
            rescanAll();
        }
    }

    public void removeBlock(String blockId) {
        if (selectedBlocks.remove(blockId)) {
            InferedClientHolder.config.setBlockSelection(id, selectedBlocks);
            rescanAll();
        }
    }

     
    public List<String> availableBlocks() {
        List<String> out = new ArrayList<>();
        for (var blockId : Registries.BLOCK.getIds()) {
            Block block = Registries.BLOCK.get(blockId);
            if (block.getDefaultState().isAir()) continue;
            if (!storageFilter.test(block)) continue;
            out.add(blockId.toString());
        }
        out.sort(String::compareTo);
        return out;
    }

    public boolean isFillOn() {
        return InferedClientHolder.config.getSetting(id + ".fill");
    }

    public void setFill(boolean value) {
        InferedClientHolder.config.setSetting(id + ".fill", value);
    }

    @Override
    public void reset() {
        super.reset();
        InferedClientHolder.config.setSetting(id + ".fill", false);
    }

     
    protected abstract boolean matches(BlockState state);

    private String idOf(BlockState state) {
        return Registries.BLOCK.getId(state.getBlock()).toString();
    }

     

     




    @Override
    public void onEnable() {
        rescanAll();
    }

     
    public void onChunkLoad(WorldChunk chunk, int bottomY) {
        if (!enabled) return;
        Set<String> wanted = new HashSet<>(selectedBlocks);
        if (wanted.isEmpty()) return;
        WORKER.submit(() -> {
            if (!enabled) return;
            Chunk sc = searchChunk(chunk, wanted, bottomY);
            synchronized (chunks) {
                long key = ChunkPos.toLong(sc.x, sc.z);
                if (sc.size() > 0) {
                    chunks.put(key, sc);
                } else {
                    chunks.remove(key);
                }
                groupsDirty = true;
            }
        });
    }

    public void onChunkUnload(WorldChunk chunk) {
        synchronized (chunks) {
            if (chunks.remove(ChunkPos.toLong(chunk.getPos().x, chunk.getPos().z)) != null) {
                groupsDirty = true;
            }
        }
    }

     
    private void removeFromChunk(Chunk c, BlockPos pos) {
        if (!c.remove(pos)) return;
        if (c.size() == 0) {
            chunks.remove(ChunkPos.toLong(c.x, c.z));
        }
    }

    public void onBlockUpdate(BlockPos pos, BlockState oldState, BlockState newState) {
        if (!enabled) return;
        boolean was = matches(oldState);
        boolean now = matches(newState);
         
         
        if (!was && !now) return;

        long chunkKey = ChunkPos.toLong(pos.getX() >> 4, pos.getZ() >> 4);
        synchronized (chunks) {
            Chunk c = chunks.get(chunkKey);
            if (was && c != null) removeFromChunk(c, pos);
            if (now) {
                c = chunks.get(chunkKey);
                if (c == null) {
                    c = new Chunk(pos.getX() >> 4, pos.getZ() >> 4);
                    chunks.put(chunkKey, c);
                }
                c.add(pos, newState.getBlock());
            }
            groupsDirty = true;
        }
    }

    public void clear() {
        synchronized (chunks) {
            chunks.clear();
            groupsDirty = true;
        }
        renderBlocks = new Long2ObjectOpenHashMap<>();
        renderGroups = List.of();
    }

     
    public void rescanAll() {
        MinecraftClient client = MinecraftClient.getInstance();
        if (client.world == null || client.player == null) return;
        Set<String> wanted = new HashSet<>(selectedBlocks);
        synchronized (chunks) {
            chunks.clear();
            groupsDirty = true;
        }
        if (wanted.isEmpty()) return;
         
         
        int pcx = client.player.getChunkPos().x;
        int pcz = client.player.getChunkPos().z;
        int viewDist = Math.max(2, client.options.getViewDistance().getValue());
        int bottomY = client.world.getBottomY();
        for (int cx = pcx - viewDist; cx <= pcx + viewDist; cx++) {
            for (int cz = pcz - viewDist; cz <= pcz + viewDist; cz++) {
                WorldChunk chunk = (WorldChunk) client.world.getChunk(cx, cz, net.minecraft.world.chunk.ChunkStatus.FULL, false);
                if (chunk == null || chunk.isEmpty()) continue;
                WORKER.submit(() -> {
                    if (!enabled) return;
                    Chunk sc = searchChunk(chunk, wanted, bottomY);
                    synchronized (chunks) {
                        long key = ChunkPos.toLong(sc.x, sc.z);
                        if (sc.size() > 0) {
                            chunks.put(key, sc);
                        } else {
                             
                             
                             
                            chunks.remove(key);
                        }
                        groupsDirty = true;
                    }
                });
            }
        }
    }

     

    private volatile boolean groupsDirty;
    private volatile List<Group> renderGroups = List.of();

    @Override
    public void tick(MinecraftClient client) {
        if (!enabled) {
            if (!chunks.isEmpty()) clear();
            return;
        }
        if (client.world == null) {
            clear();
            return;
        }

        if (groupsDirty) {
            rebuildGroups();
            groupsDirty = false;
        }
    }

    @Override
    public void onDisable() {
        clear();
    }

     
    private void rebuildGroups() {
        Long2ObjectMap<EspBlock> all = new Long2ObjectOpenHashMap<>();
        synchronized (chunks) {
            for (Chunk c : chunks.values()) {
                if (c.blocks == null) continue;
                for (EspBlock b : c.blocks.values()) all.put(b.key(), b);
            }
        }

        Map<EspBlock, EspBlock> parent = new IdentityHashMap<>();
        for (EspBlock b : all.values()) parent.put(b, b);

         
        int[][] dirs = {{1,0,0},{-1,0,0},{0,1,0},{0,-1,0},{0,0,1},{0,0,-1}};
        for (EspBlock b : all.values()) {
            for (int[] d : dirs) {
                EspBlock n = all.get(EspBlock.key(b.x + d[0], b.y + d[1], b.z + d[2]));
                if (n != null && n.block == b.block) union(parent, b, n);
            }
        }

        List<Group> newGroups = new ArrayList<>();
        Map<EspBlock, Group> rootToGroup = new IdentityHashMap<>();
        for (EspBlock b : all.values()) {
            EspBlock root = find(parent, b);
            Group g = rootToGroup.get(root);
            if (g == null) {
                g = new Group();
                rootToGroup.put(root, g);
                newGroups.add(g);
            }
            g.add(b);
        }

        renderBlocks = all;
        groups.clear();
        groups.addAll(newGroups);
        renderGroups = List.copyOf(newGroups);
    }

    private static EspBlock find(Map<EspBlock, EspBlock> parent, EspBlock b) {
        EspBlock root = b;
        while (parent.get(root) != root) root = parent.get(root);
        while (parent.get(b) != root) {
            EspBlock next = parent.get(b);
            parent.put(b, root);
            b = next;
        }
        return root;
    }

    private static void union(Map<EspBlock, EspBlock> parent, EspBlock a, EspBlock b) {
        EspBlock ra = find(parent, a), rb = find(parent, b);
        if (ra != rb) parent.put(ra, rb);
    }

     

    private Chunk searchChunk(WorldChunk chunk, Set<String> wanted, int bottomY) {
        Chunk sc = new Chunk(chunk.getPos().x, chunk.getPos().z);
        ChunkSection[] sections = chunk.getSectionArray();

        for (int s = 0; s < sections.length; s++) {
            ChunkSection section = sections[s];
            if (section == null || section.isEmpty()) continue;
             
             
             
            if (!section.getBlockStateContainer().hasAny(
                    state -> wanted.contains(Registries.BLOCK.getId(state.getBlock()).toString()))) {
                continue;
            }

            int baseY = bottomY + (s << 4);
            for (int x = 0; x < 16; x++) {
                for (int z = 0; z < 16; z++) {
                    for (int y = 0; y < 16; y++) {
                        BlockState state = section.getBlockState(x, y, z);
                        if (wanted.contains(Registries.BLOCK.getId(state.getBlock()).toString())) {
                            sc.add(new BlockPos((chunk.getPos().x << 4) + x, baseY + y, (chunk.getPos().z << 4) + z),
                                state.getBlock());
                        }
                    }
                }
            }
        }
        return sc;
    }

     

    @Override
    public void renderLast(WorldRenderContext ctx) {
        if (!enabled || renderGroups.isEmpty()) return;
        int color = getColor();
        int fillColor = getFillColor();
        boolean fill = isFillOn();

        for (Group g : renderGroups) {
            for (EspBlock b : g.blocks) {
                renderBlock(ctx, b, color, fillColor, fill);
            }
        }
    }

     
    private void renderBlock(WorldRenderContext ctx, EspBlock b, int color, int fillColor, boolean fill) {
         
         
        boolean eFront = isSameGroup(b, 0, 0, 1);
        boolean eBack  = isSameGroup(b, 0, 0, -1);
        boolean eLeft  = isSameGroup(b, -1, 0, 0);
        boolean eRight = isSameGroup(b, 1, 0, 0);
        boolean eTop   = isSameGroup(b, 0, 1, 0);
        boolean eBottom= isSameGroup(b, 0, -1, 0);

        double x1 = b.x, y1 = b.y, z1 = b.z;
        double x2 = b.x + 1, y2 = b.y + 1, z2 = b.z + 1;

        if (fill) {
            if (!eFront) InferedDrawer.drawQuad(ctx, x1, y1, z2, x2, y1, z2, x2, y2, z2, x1, y2, z2, fillColor);
            if (!eBack)  InferedDrawer.drawQuad(ctx, x1, y1, z1, x2, y1, z1, x2, y2, z1, x1, y2, z1, fillColor);
            if (!eBottom)InferedDrawer.drawQuad(ctx, x1, y1, z1, x2, y1, z1, x2, y1, z2, x1, y1, z2, fillColor);
            if (!eTop)   InferedDrawer.drawQuad(ctx, x1, y2, z1, x2, y2, z1, x2, y2, z2, x1, y2, z2, fillColor);
            if (!eLeft)  InferedDrawer.drawQuad(ctx, x1, y1, z1, x1, y1, z2, x1, y2, z2, x1, y2, z1, fillColor);
            if (!eRight) InferedDrawer.drawQuad(ctx, x2, y1, z1, x2, y1, z2, x2, y2, z2, x2, y2, z1, fillColor);
        }

         
         
         
        if (!eLeft && !eBack)  InferedDrawer.drawLine(ctx, v(x1, y1, z1), v(x1, y2, z1), color);
        if (!eLeft && !eFront) InferedDrawer.drawLine(ctx, v(x1, y1, z2), v(x1, y2, z2), color);
        if (!eRight && !eBack) InferedDrawer.drawLine(ctx, v(x2, y1, z1), v(x2, y2, z1), color);
        if (!eRight && !eFront)InferedDrawer.drawLine(ctx, v(x2, y1, z2), v(x2, y2, z2), color);
         
        if (!eBack && !eBottom)  InferedDrawer.drawLine(ctx, v(x1, y1, z1), v(x2, y1, z1), color);
        if (!eFront && !eBottom) InferedDrawer.drawLine(ctx, v(x1, y1, z2), v(x2, y1, z2), color);
        if (!eLeft && !eBottom)  InferedDrawer.drawLine(ctx, v(x1, y1, z1), v(x1, y1, z2), color);
        if (!eRight && !eBottom) InferedDrawer.drawLine(ctx, v(x2, y1, z1), v(x2, y1, z2), color);
         
        if (!eBack && !eTop)  InferedDrawer.drawLine(ctx, v(x1, y2, z1), v(x2, y2, z1), color);
        if (!eFront && !eTop) InferedDrawer.drawLine(ctx, v(x1, y2, z2), v(x2, y2, z2), color);
        if (!eLeft && !eTop)  InferedDrawer.drawLine(ctx, v(x1, y2, z1), v(x1, y2, z2), color);
        if (!eRight && !eTop) InferedDrawer.drawLine(ctx, v(x2, y2, z1), v(x2, y2, z2), color);
    }

    private static Vec3d v(double x, double y, double z) {
        return new Vec3d(x, y, z);
    }

    private boolean isSameGroup(EspBlock b, int dx, int dy, int dz) {
        EspBlock n = renderBlocks.get(EspBlock.key(b.x + dx, b.y + dy, b.z + dz));
        return n != null && n.group == b.group && n.block == b.block;
    }

     
    protected abstract int getColor();

     
    protected int getFillColor() {
        int color = getColor();
         
        return (color & 0x00FFFFFF) | 0x60000000;  
    }
}
