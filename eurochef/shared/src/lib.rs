// TODO: EDB-specific code should probably be part of eurochef-edb
pub mod entities;
pub mod maps;
pub mod platform;
pub mod spreadsheets;
pub mod textures;

pub struct IdentifiableResult<T> {
    pub hashcode: u32,
    pub data: anyhow::Result<T>,
}

impl<T> IdentifiableResult<T> {
    pub fn new(hashcode: u32, data: anyhow::Result<T>) -> Self {
        Self { hashcode, data }
    }
}
