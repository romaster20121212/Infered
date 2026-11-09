package infered.infered.client.modules;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.reflect.TypeToken;
import infered.infered.client.InferedClientHolder;
import infered.infered.client.render.InferedToast;
import net.fabricmc.fabric.api.client.rendering.v1.WorldRenderContext;
import net.minecraft.block.entity.BarrelBlockEntity;
import net.minecraft.block.entity.BlockEntity;
import net.minecraft.block.entity.ChestBlockEntity;
import net.minecraft.block.entity.DispenserBlockEntity;
import net.minecraft.block.entity.EnderChestBlockEntity;
import net.minecraft.block.entity.FurnaceBlockEntity;
import net.minecraft.block.entity.HopperBlockEntity;
import net.minecraft.block.entity.ShulkerBoxBlockEntity;
import net.minecraft.block.entity.TrappedChestBlockEntity;
import net.minecraft.client.MinecraftClient;
import net.minecraft.client.sound.PositionedSoundInstance;
import net.minecraft.item.Items;
import net.minecraft.sound.SoundEvents;
import net.minecraft.text.Text;
import net.minecraft.util.math.BlockPos;
import net.minecraft.util.math.ChunkPos;
import net.minecraft.world.chunk.WorldChunk;

import java.io.IOException;
import java.lang.reflect.Type;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.HashMap;

 






public class StashFinder extends Module {
    private static final Gson GSON = new GsonBuilder().setPrettyPrinting().create();
    private static final Type CHUNK_LIST = new TypeToken<List<StashChunk>>() {}.getType();

    public final List<StashChunk> stashes = new ArrayList<>();

    public int minStorageCount = 4;
    public boolean notificationsOn = true;

    private final Map<Long, Double> tracerTargets = new HashMap<>();

    public StashFinder() {
        super("stashfinder", "Stash Finder", "Finds and records chunks dense with storage blocks");
        load();
    }

     

    public int getMinStorageCount() { return minStorageCount; }

    public void setMinStorageCount(int value) {
        minStorageCount = Math.max(1, value);
        InferedClientHolder.config.setInt(id + ".minstorages", minStorageCount);
    }

    public boolean isNotificationsOn() { return notificationsOn; }

    public void setNotificationsOn(boolean value) {
        notificationsOn = value;
        InferedClientHolder.config.setSetting(id + ".notifications", value);
    }

     
    public void loadSettings() {
         
        notificationsOn = InferedClientHolder.config.getSetting(id + ".notifications", true);
        minStorageCount = InferedClientHolder.config.getInt(id + ".minstorages", 4);
    }

     



    @Override
    public void onEnable() {
        MinecraftClient client = MinecraftClient.getInstance();
        if (client.world == null || client.player == null) return;
        int pcx = client.player.getChunkPos().x;
        int pcz = client.player.getChunkPos().z;
        int viewDist = Math.max(2, client.options.getViewDistance().getValue());
        for (int cx = pcx - viewDist; cx <= pcx + viewDist; cx++) {
            for (int cz = pcz - viewDist; cz <= pcz + viewDist; cz++) {
                WorldChunk chunk = (WorldChunk) client.world.getChunk(cx, cz,
                    net.minecraft.world.chunk.ChunkStatus.FULL, false);
                if (chunk == null || chunk.isEmpty()) continue;
                onChunkLoad(chunk);
            }
        }
    }

    public void clearStashes() {
        stashes.clear();
        tracerTargets.clear();
        saveJson();
        saveCsv();
    }

     

    public void onChunkLoad(WorldChunk chunk) {
        if (!enabled) return;
        StashChunk found = scanChunk(chunk);
        if (found == null) return;

        int idx = stashes.indexOf(found);
        boolean isNew = idx < 0;
        StashChunk prev = isNew ? null : stashes.get(idx);
        boolean changed = isNew || prev == null || !prev.countsEqual(found);

        if (isNew) stashes.add(found);
        else stashes.set(idx, found);

        if (changed) {
            MinecraftClient client = MinecraftClient.getInstance();
            if (client.player != null) {
                tracerTargets.put(found.chunkKey, (double) client.player.getEyeY());
            }
            saveJson();
            saveCsv();

            if (notificationsOn) {
                InferedToast.show("Stash found",
                    found.total() + " storages at " + found.middleX + ", " + found.middleZ);
            }
        }
    }

    private StashChunk scanChunk(WorldChunk chunk) {
        int chests = 0, barrels = 0, shulkers = 0, enderChests = 0,
            furnaces = 0, dispensers = 0, hoppers = 0;

        for (BlockEntity be : chunk.getBlockEntities().values()) {
             
            if (be instanceof TrappedChestBlockEntity) chests++;
            else if (be instanceof ChestBlockEntity) chests++;
            else if (be instanceof BarrelBlockEntity) barrels++;
            else if (be instanceof ShulkerBoxBlockEntity) shulkers++;
            else if (be instanceof EnderChestBlockEntity) enderChests++;
            else if (be instanceof FurnaceBlockEntity) furnaces++;
            else if (be instanceof DispenserBlockEntity) dispensers++;
            else if (be instanceof HopperBlockEntity) hoppers++;
        }

        int total = chests + barrels + shulkers + enderChests + furnaces + dispensers + hoppers;
        if (total < minStorageCount) return null;

        ChunkPos pos = chunk.getPos();
        return new StashChunk(ChunkPos.toLong(pos.x, pos.z),
            chests, barrels, shulkers, enderChests, furnaces, dispensers, hoppers);
    }

     

    @Override
    public void tick(MinecraftClient client) {
         
        if (!tracerTargets.isEmpty() && client.player != null) {
            tracerTargets.entrySet().removeIf(e -> {
                double x = ChunkPos.getPackedX(e.getKey());
                double z = ChunkPos.getPackedZ(e.getKey());
                double dx = x + 8 - client.player.getX();
                double dz = z + 8 - client.player.getZ();
                return dx * dx + dz * dz < 12 * 12;
            });
        }
    }

     

    @Override
    public void renderLast(WorldRenderContext ctx) {
        if (!enabled || tracerTargets.isEmpty()) return;
        MinecraftClient client = MinecraftClient.getInstance();
        if (client.player == null) return;

        int gold = 0xFFFFD700;
        for (var entry : tracerTargets.entrySet()) {
            double x = ChunkPos.getPackedX(entry.getKey()) + 8;
            double z = ChunkPos.getPackedZ(entry.getKey()) + 8;
            double y = client.player.getEyeY();
            infered.infered.client.render.InferedDrawer.drawTracer(ctx, new net.minecraft.util.math.Vec3d(x, y, z), gold);
             
            double x0 = x - 8, z0 = z - 8, x1 = x + 8, z1 = z + 8;
            double y0 = client.world.getBottomY() + 1;
            infered.infered.client.render.InferedDrawer.drawLine(ctx, new net.minecraft.util.math.Vec3d(x0, y0, z0), new net.minecraft.util.math.Vec3d(x1, y0, z0), gold);
            infered.infered.client.render.InferedDrawer.drawLine(ctx, new net.minecraft.util.math.Vec3d(x1, y0, z0), new net.minecraft.util.math.Vec3d(x1, y0, z1), gold);
            infered.infered.client.render.InferedDrawer.drawLine(ctx, new net.minecraft.util.math.Vec3d(x1, y0, z1), new net.minecraft.util.math.Vec3d(x0, y0, z1), gold);
            infered.infered.client.render.InferedDrawer.drawLine(ctx, new net.minecraft.util.math.Vec3d(x0, y0, z1), new net.minecraft.util.math.Vec3d(x0, y0, z0), gold);
        }
    }

     

    private Path stashDir() {
        MinecraftClient client = MinecraftClient.getInstance();
        String world = "unknown";
        if (client.getServer() != null) {
            world = client.getServer().getSaveProperties().getLevelName();
        } else if (client.getCurrentServerEntry() != null && client.getCurrentServerEntry().address != null) {
            world = client.getCurrentServerEntry().address;
        }
        return net.fabricmc.loader.api.FabricLoader.getInstance().getGameDir()
            .resolve("infered").resolve("stashes").resolve(sanitize(world));
    }

    private static String sanitize(String s) {
        return s.replaceAll("[^a-zA-Z0-9._-]", "_");
    }

    public void load() {
        try {
            Path json = stashDir().resolve("stashes.json");
            if (Files.exists(json)) {
                List<StashChunk> loaded = GSON.fromJson(Files.readString(json), CHUNK_LIST);
                if (loaded != null) stashes.addAll(loaded);
            }
        } catch (Exception ignored) {
        }
    }

    public void saveJson() {
        try {
            Path json = stashDir().resolve("stashes.json");
            Files.createDirectories(json.getParent());
            Files.writeString(json, GSON.toJson(stashes));
        } catch (IOException ignored) {
        }
    }

    public void saveCsv() {
        try {
            Path csv = stashDir().resolve("stashes.csv");
            Files.createDirectories(csv.getParent());
            StringBuilder sb = new StringBuilder("x,z,total,chests,barrels,shulkers,enderChests,furnaces,dispensers,hoppers\n");
            for (StashChunk c : stashes) {
                sb.append(c.middleX).append(',').append(c.middleZ).append(',').append(c.total())
                    .append(',').append(c.chests).append(',').append(c.barrels).append(',')
                    .append(c.shulkers).append(',').append(c.enderChests).append(',')
                    .append(c.furnaces).append(',').append(c.dispensers).append(',')
                    .append(c.hoppers).append('\n');
            }
            Files.writeString(csv, sb.toString());
        } catch (IOException ignored) {
        }
    }

     
    public static class StashChunk {
        public transient long chunkKey;
        public int middleX, middleZ;
        public int chests, barrels, shulkers, enderChests, furnaces, dispensers, hoppers;

        public StashChunk() {
            chunkKey = ChunkPos.toLong(middleX - 8, middleZ - 8);
        }

        public StashChunk(long chunkKey, int chests, int barrels, int shulkers,
                          int enderChests, int furnaces, int dispensers, int hoppers) {
            this.chunkKey = chunkKey;
            this.middleX = (int) ChunkPos.getPackedX(chunkKey) + 8;
            this.middleZ = (int) ChunkPos.getPackedZ(chunkKey) + 8;
            this.chests = chests;
            this.barrels = barrels;
            this.shulkers = shulkers;
            this.enderChests = enderChests;
            this.furnaces = furnaces;
            this.dispensers = dispensers;
            this.hoppers = hoppers;
        }

        public int total() {
            return chests + barrels + shulkers + enderChests + furnaces + dispensers + hoppers;
        }

        public boolean countsEqual(StashChunk o) {
            return o != null
                && chests == o.chests && barrels == o.barrels && shulkers == o.shulkers
                && enderChests == o.enderChests && furnaces == o.furnaces
                && dispensers == o.dispensers && hoppers == o.hoppers;
        }

        @Override
        public boolean equals(Object o) {
            return o instanceof StashChunk c && c.middleX == middleX && c.middleZ == middleZ;
        }

        @Override
        public int hashCode() {
            return middleX * 31 + middleZ;
        }
    }
}
