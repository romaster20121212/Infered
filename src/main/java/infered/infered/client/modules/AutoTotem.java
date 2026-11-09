package infered.infered.client.modules;

import net.minecraft.client.MinecraftClient;
import net.minecraft.item.Items;
import net.minecraft.screen.PlayerScreenHandler;
import net.minecraft.entity.player.PlayerInventory;
import net.minecraft.screen.slot.SlotActionType;

 


public class AutoTotem extends Module {
    private int cooldown = 0;

    public AutoTotem() {
        super("autototem", "AutoTotem", "Automatically equips totems in off-hand", false);
    }

    @Override
    public void tick(MinecraftClient client) {
        if (!enabled || client.player == null || client.world == null) return;
        
         
        if (cooldown > 0) {
            cooldown--;
            return;
        }

         
        if (!(client.player.currentScreenHandler instanceof PlayerScreenHandler)) {
            return;
        }

        PlayerInventory inventory = client.player.getInventory();
        
         
        if (inventory.offHand.get(0).getItem() == Items.TOTEM_OF_UNDYING) {
            return;
        }

         
        int totemSlot = findTotemSlot(inventory);
        if (totemSlot == -1) return;

         
        int menuSlot = totemSlot < 9 ? totemSlot + 36 : totemSlot;

         
        if (client.interactionManager != null) {
            client.interactionManager.clickSlot(
                client.player.currentScreenHandler.syncId,
                menuSlot,
                40,  
                SlotActionType.SWAP,
                client.player
            );
            cooldown = 5;  
        }
    }

    private int findTotemSlot(PlayerInventory inventory) {
         
        for (int i = 0; i < 36; i++) {
            if (inventory.getStack(i).getItem() == Items.TOTEM_OF_UNDYING) {
                return i;
            }
        }
        return -1;
    }
}