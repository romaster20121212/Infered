package infered.infered.client.modules;

import net.minecraft.client.MinecraftClient;
import net.minecraft.entity.EquipmentSlot;
import net.minecraft.item.ArmorItem;
import net.minecraft.screen.PlayerScreenHandler;
import net.minecraft.entity.player.PlayerInventory;
import net.minecraft.screen.slot.SlotActionType;

 


public class AutoArmor extends Module {
    private int cooldown = 0;

    public AutoArmor() {
        super("autoarmor", "AutoArmor", "Automatically equips armor from inventory", false);
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
        
         
        EquipmentSlot[] armorSlots = {EquipmentSlot.HEAD, EquipmentSlot.CHEST, EquipmentSlot.LEGS, EquipmentSlot.FEET};
        for (EquipmentSlot slot : armorSlots) {
            int armorSlotIndex = getArmorSlotIndex(slot);
            if (armorSlotIndex == -1) continue;
            
             
            if (!inventory.getStack(armorSlotIndex).isEmpty()) continue;
            
             
            int armorPieceSlot = findArmorForSlot(inventory, slot);
            if (armorPieceSlot == -1) continue;
            
             
            if (client.interactionManager != null) {
                int menuSlot = armorPieceSlot < 9 ? armorPieceSlot + 36 : armorPieceSlot;
                client.interactionManager.clickSlot(
                    client.player.currentScreenHandler.syncId,
                    menuSlot,
                    0,
                    SlotActionType.QUICK_MOVE,
                    client.player
                );
                cooldown = 5;  
                return;  
            }
        }
    }

    private int getArmorSlotIndex(EquipmentSlot slot) {
        if (slot == EquipmentSlot.HEAD) return 39;
        if (slot == EquipmentSlot.CHEST) return 38;
        if (slot == EquipmentSlot.LEGS) return 37;
        if (slot == EquipmentSlot.FEET) return 36;
        return -1;
    }

    private int findArmorForSlot(PlayerInventory inventory, EquipmentSlot slot) {
         
        for (int i = 0; i < 36; i++) {
            var stack = inventory.getStack(i);
            if (stack.isEmpty()) continue;
            
            if (stack.getItem() instanceof ArmorItem) {
                 
                 
                return i;
            }
        }
        
        return -1;
    }
}