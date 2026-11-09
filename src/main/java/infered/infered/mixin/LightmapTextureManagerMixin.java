package infered.infered.mixin;

import infered.infered.client.modules.Fullbright;
import net.minecraft.client.option.SimpleOption;
import net.minecraft.client.render.LightmapTextureManager;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Redirect;

 














@Mixin(LightmapTextureManager.class)
public abstract class LightmapTextureManagerMixin {
    private static final double FULLBRIGHT_GAMMA = 16.0;

    @Redirect(
        method = "update",
        at = @At(
            value = "INVOKE",
            target = "Lnet/minecraft/client/option/SimpleOption;getValue()Ljava/lang/Object;",
            ordinal = 1
        )
    )
    private Object infered$fullbrightGamma(SimpleOption<Double> gammaOption) {
        if (Fullbright.isForcing()) {
            return FULLBRIGHT_GAMMA;
        }
        return gammaOption.getValue();
    }
}
