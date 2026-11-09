package infered.infered;

import net.fabricmc.api.ModInitializer;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public class Infered implements ModInitializer {
    public static final String MOD_ID = "infered";
    public static final Logger LOGGER = LoggerFactory.getLogger(MOD_ID);

    @Override
    public void onInitialize() {
        LOGGER.info("Infered mod loaded successfully!");
    }
}
