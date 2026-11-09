package infered.infered.client;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import net.fabricmc.loader.api.FabricLoader;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Map;
import java.util.Set;

 



public class InferedConfig {
    private static final Gson GSON = new GsonBuilder().setPrettyPrinting().create();
    private static final Path PATH = FabricLoader.getInstance().getConfigDir().resolve("infered.json");

    public Set<String> disabledModules = new HashSet<>();
    public Map<String, Boolean> moduleSettings = new HashMap<>();
    public Map<String, Integer> intSettings = new HashMap<>();
    public Map<String, Integer> colorSettings = new HashMap<>();
    public Set<String> blockEspBlocks = new HashSet<>();
    public Set<String> storageEspBlocks = new HashSet<>();

    public void setBlockSelection(String moduleId, Set<String> blocks) {
        if ("blockesp".equals(moduleId)) {
            blockEspBlocks = blocks;
        } else if ("storageesp".equals(moduleId)) {
            storageEspBlocks = blocks;
        }
        save();
    }

    public Set<String> getBlockSelection(String moduleId) {
        if ("storageesp".equals(moduleId)) {
            return storageEspBlocks;
        }
        return blockEspBlocks;
    }

    public static InferedConfig load() {
        try {
            if (Files.exists(PATH)) {
                InferedConfig cfg = GSON.fromJson(Files.readString(PATH), InferedConfig.class);
                if (cfg != null) {
                    if (cfg.disabledModules == null) cfg.disabledModules = new HashSet<>();
                    if (cfg.moduleSettings == null) cfg.moduleSettings = new HashMap<>();
                    if (cfg.intSettings == null) cfg.intSettings = new HashMap<>();
                    if (cfg.colorSettings == null) cfg.colorSettings = new HashMap<>();
                    if (cfg.blockEspBlocks == null) cfg.blockEspBlocks = new HashSet<>();
                    if (cfg.storageEspBlocks == null) cfg.storageEspBlocks = new HashSet<>();
                    return cfg;
                }
            }
        } catch (Exception ignored) {
        }
        return new InferedConfig();
    }

    public void save() {
        try {
            Files.createDirectories(PATH.getParent());
            Files.writeString(PATH, GSON.toJson(this));
        } catch (IOException ignored) {
        }
    }

    public boolean isEnabled(String moduleId) {
        return !disabledModules.contains(moduleId);
    }

    public void setEnabled(String moduleId, boolean enabled) {
        if (enabled) disabledModules.remove(moduleId);
        else disabledModules.add(moduleId);
        save();
    }

    public boolean getSetting(String key) {
        return moduleSettings.getOrDefault(key, false);
    }

     
    public boolean getSetting(String key, boolean defaultValue) {
        Boolean v = moduleSettings.get(key);
        return v != null ? v : defaultValue;
    }

    public int getInt(String key, int defaultValue) {
        Integer v = intSettings.get(key);
        return v != null ? v : defaultValue;
    }

    public void setInt(String key, int value) {
        intSettings.put(key, value);
        save();
    }

    public int getColor(String key, int defaultValue) {
        Integer v = colorSettings.get(key);
        return v != null ? v : defaultValue;
    }

    public void setColor(String key, int value) {
        colorSettings.put(key, value);
        save();
    }

    public void setSetting(String key, boolean value) {
        moduleSettings.put(key, value);
        save();
    }
}
