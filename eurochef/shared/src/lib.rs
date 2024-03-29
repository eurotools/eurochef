use anyhow::anyhow;

pub mod entities;
pub mod filesystem;
pub mod hashcodes;
pub mod maps;
pub mod platform;
pub mod script;
pub mod spreadsheets;
pub mod textures;

pub struct IdentifiableResult<T: Clone> {
    pub hashcode: u32,
    pub data: anyhow::Result<T>,
}

impl<T: Clone> IdentifiableResult<T> {
    pub fn new(hashcode: u32, data: anyhow::Result<T>) -> Self {
        Self { hashcode, data }
    }
}

impl<T: Clone> Clone for IdentifiableResult<T> {
    fn clone(&self) -> Self {
        Self {
            hashcode: self.hashcode,
            data: match &self.data {
                Ok(d) => Ok(d.clone()),
                Err(e) => Err(anyhow!(e.to_string())), // Dirty but fine for our purposes
            },
        }
    }
}
