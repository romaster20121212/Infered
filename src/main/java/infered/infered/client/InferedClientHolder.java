package infered.infered.client;

import infered.infered.Infered;
import infered.infered.client.modules.ModuleManager;

 


public final class InferedClientHolder {
    public static InferedConfig config;
    public static ModuleManager manager;

    static void init() {
        if (config == null) {
            config = InferedConfig.load();
        }
        if (manager == null) {
            manager = ModuleManager.get();
        }
        Infered.LOGGER.info("Infered config loaded ({} modules disabled)",
            config.disabledModules.size());
    }
}
