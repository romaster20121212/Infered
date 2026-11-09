 
 
 
 
 

 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
     
    None,
     
    Chat,
     
    Inventory,
     
    Container,
     
    Crafting,
     
    Menu,
     
    Disconnected,
     
    Unknown,
}

impl Screen {
     
    pub fn is_open(self) -> bool {
        self != Screen::None
    }
}
