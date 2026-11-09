use std::io;
use thiserror::Error;

 
#[derive(Error, Debug)]
pub enum HookError {
     
    #[error("invalid parameter")]
    InvalidParameter,

     
    #[error("memory protect error, code:{0}")]
    MemoryProtect(u32),

     
    #[error("memory allocation error, code:{0}")]
    MemoryAllocation(u32),

     
    #[error("searching memory failed")]
    MemorySearching,

     
    #[error("memory layout format error")]
    MemoryLayoutFormat,

     
    #[error("disassemble error")]
    Disassemble,

     
    #[error("moving code error")]
    MoveCode,

     
    #[error("not supported moving code")]
    MovingCodeNotSupported,

     
    #[error("pre hook failed")]
    PreHook,

     
    #[error("io error")]
    Io(#[from] io::Error),

     
    #[error("unknown error")]
    Unknown,
}
