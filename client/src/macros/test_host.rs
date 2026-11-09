 
 
 
 
 

use infered_macros::MovementKey;

use super::host::{Host, Snapshot};

 
#[derive(Debug)]
pub struct FakeHost {
    pub snapshot: Snapshot,
    pub taps: Vec<String>,
    pub held: Vec<(MovementKey, bool)>,
    pub use_item: Vec<bool>,
    pub attacks: u32,
    pub interacts: u32,
    pub picks: u32,
    pub hotbar: Vec<u8>,
    pub chats: Vec<String>,
    pub logs: Vec<String>,
    pub inventory_opens: u32,
    pub screens_closed: u32,
}

impl Default for FakeHost {
    fn default() -> Self {
        FakeHost {
            snapshot: Snapshot {
                available: true,
                in_world: true,
                health: 20.0,
                max_health: 20.0,
                food: 20.0,
                on_ground: true,
                slot: 0,
                ..Snapshot::default()
            },
            taps: Vec::new(),
            held: Vec::new(),
            use_item: Vec::new(),
            attacks: 0,
            interacts: 0,
            picks: 0,
            hotbar: Vec::new(),
            chats: Vec::new(),
            logs: Vec::new(),
            inventory_opens: 0,
            screens_closed: 0,
        }
    }
}

impl Host for FakeHost {
    fn snapshot(&mut self) -> Snapshot {
        self.snapshot.clone()
    }

    fn tap_key(&mut self, key: &str) -> anyhow::Result<()> {
        self.taps.push(key.to_string());
        Ok(())
    }

    fn set_movement(&mut self, key: MovementKey, down: bool) -> anyhow::Result<()> {
        self.held.push((key, down));
        Ok(())
    }

    fn set_use(&mut self, down: bool) -> anyhow::Result<()> {
        self.use_item.push(down);
        Ok(())
    }

    fn attack(&mut self) -> anyhow::Result<()> {
        self.attacks += 1;
        Ok(())
    }

    fn interact(&mut self) -> anyhow::Result<()> {
        self.interacts += 1;
        Ok(())
    }

    fn pick_block(&mut self) -> anyhow::Result<()> {
        self.picks += 1;
        Ok(())
    }

    fn hotbar(&mut self, slot: u8) -> anyhow::Result<()> {
        self.hotbar.push(slot);
        self.snapshot.slot = slot.saturating_sub(1) as i32;
        Ok(())
    }

    fn selected_slot(&mut self) -> anyhow::Result<i32> {
        Ok(self.snapshot.slot)
    }

    fn open_inventory(&mut self) -> anyhow::Result<()> {
        self.inventory_opens += 1;
        Ok(())
    }

    fn close_screen(&mut self) -> anyhow::Result<()> {
        self.screens_closed += 1;
        Ok(())
    }

    fn send_chat(&mut self, message: &str) -> anyhow::Result<()> {
        self.chats.push(message.to_string());
        Ok(())
    }

    fn log(&mut self, message: &str) {
        self.logs.push(message.to_string());
    }
}
