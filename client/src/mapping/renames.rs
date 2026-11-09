 
 
 
 
 
 
 
 
 

use crate::mapping::minecraft_version::MinecraftVersion;
use crate::mapping::MinecraftClassType;

 
const fn v(major: u32, minor: u32, patch: u32) -> MinecraftVersion {
    MinecraftVersion::new(major, minor, patch)
}

 
 
 
 
enum Rename {
     
     
     
    Class {
        class: MinecraftClassType,
        legacy: &'static [(MinecraftVersion, &'static str)],
    },
     
    Member {
        owner: MinecraftClassType,
        canonical: &'static str,
        legacy: &'static [(MinecraftVersion, &'static str)],
    },
}

 
static RENAMES: &[Rename] = &[
     
    Rename::Member {
        owner: MinecraftClassType::Window,
        canonical: "handle",
        legacy: &[(v(1, 21, 9), "getWindow")],
    },
     
    Rename::Member {
        owner: MinecraftClassType::MultiPlayerGameMode,
        canonical: "handleContainerInput",
        legacy: &[(v(26, 1, 0), "handleInventoryMouseClick")],
    },
     
    Rename::Class {
        class: MinecraftClassType::ContainerInput,
        legacy: &[(v(26, 1, 0), "net/minecraft/world/inventory/ClickType")],
    },
];

 
 
fn pick<'a>(
    current: &'a str,
    legacy: &[(MinecraftVersion, &'static str)],
    version: MinecraftVersion,
) -> &'a str {
    for &(replaced_in, old_name) in legacy {
        if version < replaced_in {
            return old_name;
        }
    }
    current
}

 
 
pub fn resolve_class(name: &str, version: MinecraftVersion) -> &str {
     
    if version == MinecraftVersion::LATEST {
        return name;
    }
    for rename in RENAMES {
        if let Rename::Class { class, legacy } = rename {
            if class.get_name() == name {
                return pick(name, legacy, version);
            }
        }
    }
    name
}

 
 
pub fn resolve_member(
    owner: MinecraftClassType,
    canonical: &str,
    version: MinecraftVersion,
) -> &str {
    if version == MinecraftVersion::LATEST {
        return canonical;
    }
    for rename in RENAMES {
        if let Rename::Member {
            owner: entry_owner,
            canonical: entry_canonical,
            legacy,
        } = rename
        {
            if *entry_owner == owner && *entry_canonical == canonical {
                return pick(canonical, legacy, version);
            }
        }
    }
    canonical
}
