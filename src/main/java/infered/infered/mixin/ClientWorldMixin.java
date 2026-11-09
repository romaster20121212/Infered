package infered.infered.mixin;

import infered.infered.client.InferedClientHolder;
import net.minecraft.block.BlockState;
import net.minecraft.client.world.ClientWorld;
import net.minecraft.util.math.BlockPos;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

 




@Mixin(ClientWorld.class)
public class ClientWorldMixin {
    @Inject(method = "updateListeners", at = @At("HEAD"))
    private void infered$onBlockUpdate(BlockPos pos, BlockState oldState, BlockState newState, int flags, CallbackInfo ci) {
        var manager = InferedClientHolder.manager;
        if (manager != null) {
            manager.onBlockUpdate(pos, oldState, newState);
        }
    }
}
