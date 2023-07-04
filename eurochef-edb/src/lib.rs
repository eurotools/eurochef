pub mod anim;
pub mod array;
pub mod common;
pub mod edb;
pub mod entity;
pub mod entity_mesh;
pub mod error;
pub mod header;
pub mod map;
pub mod script;
pub mod text;
pub mod texture;
pub mod util;
pub mod versions;

// Re-export binrw
pub use binrw;

pub type Hashcode = u32;

pub trait HashcodeUtils {
    fn is_local(&self) -> bool;
    fn base(&self) -> u32;
    fn index(&self) -> u32;
}

impl HashcodeUtils for Hashcode {
    fn is_local(&self) -> bool {
        (*self & 0x80000000) != 0
    }

    fn base(&self) -> u32 {
        *self & 0x7fff0000
    }

    fn index(&self) -> u32 {
        *self & 0x0000ffff
    }
}
