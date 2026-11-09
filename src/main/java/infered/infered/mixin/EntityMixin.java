package infered.infered.mixin;

import infered.infered.client.modules.ModuleManager;
import infered.infered.client.modules.Velocity;
import net.minecraft.entity.Entity;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.ModifyVariable;

 


@Mixin(Entity.class)
public class EntityMixin {

    @ModifyVariable(method = "setVelocity(DDD)V", at = @At("HEAD"), index = 0, argsOnly = true)
    private double modifyVelocityX(double x) {
        Velocity velocity = ModuleManager.get().velocity;
        if (velocity != null && velocity.isEnabled()) {
            return x * velocity.getHorizontal();
        }
        return x;
    }

    @ModifyVariable(method = "setVelocity(DDD)V", at = @At("HEAD"), index = 1, argsOnly = true)
    private double modifyVelocityY(double y) {
        Velocity velocity = ModuleManager.get().velocity;
        if (velocity != null && velocity.isEnabled()) {
            return y * velocity.getVertical();
        }
        return y;
    }

    @ModifyVariable(method = "setVelocity(DDD)V", at = @At("HEAD"), index = 2, argsOnly = true)
    private double modifyVelocityZ(double z) {
        Velocity velocity = ModuleManager.get().velocity;
        if (velocity != null && velocity.isEnabled()) {
            return z * velocity.getHorizontal();
        }
        return z;
    }
}