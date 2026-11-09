 
 
 
 
 
 
 
 
 
 
 

use std::sync::OnceLock;

use jni::JNIEnv;
use thiserror::Error;

use crate::mapping::client::minecraft::Minecraft;
use crate::mapping::Mapping;
use crate::module::registry::ModuleRegistry;

 
static MAPPING: OnceLock<Mapping> = OnceLock::new();
 
static CLIENT: OnceLock<Client> = OnceLock::new();

 
#[derive(Debug, Error)]
pub enum ClientError {
     
    #[error("the client is already initialized")]
    AlreadyInitialized,
     
    #[error(transparent)]
    Init(#[from] anyhow::Error),
}

 
 
pub fn init() -> Result<(), ClientError> {
    MAPPING
        .set(Mapping::new()?)
        .map_err(|_| ClientError::AlreadyInitialized)?;
    CLIENT
        .set(Client::new()?)
        .map_err(|_| ClientError::AlreadyInitialized)?;
    Ok(())
}

 
#[inline]
pub fn mapping() -> &'static Mapping {
    MAPPING.get().expect("mapping() used before client init")
}

 
#[inline]
pub fn client() -> &'static Client {
    CLIENT.get().expect("client() used before client init")
}

 
 
 
#[inline]
pub fn try_client() -> Option<&'static Client> {
    CLIENT.get()
}

 
#[inline]
pub fn minecraft() -> &'static Minecraft {
    &client().minecraft
}

 
 
#[inline]
pub fn try_minecraft() -> Option<&'static Minecraft> {
    try_client().map(|client| &client.minecraft)
}

 
pub fn env() -> anyhow::Result<JNIEnv<'static>> {
    mapping().get_env()
}

 
 
 
 
 
 
 
 
 
pub fn teardown() {
     
     
    crate::net::teardown();
    if let Some(client) = CLIENT.get() {
        client.minecraft.teardown();
    }
    if let Some(mapping) = MAPPING.get() {
        mapping.teardown();
    }
}

 
pub struct Client {
    minecraft: Minecraft,
     
    pub modules: ModuleRegistry,
}

impl Client {
     
    fn new() -> Result<Client, ClientError> {
        Ok(Client {
            minecraft: Minecraft::new()?,
            modules: ModuleRegistry::new(),
        })
    }
}
