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
pub const HC_BASE_ENTITY: Hashcode = 0x2 << 24;
pub const HC_BASE_SCRIPT: Hashcode = 0x4 << 24;
pub const HC_BASE_TEXTURE: Hashcode = 0x6 << 24;
pub const HC_BASE_PARTICLE: Hashcode = 0x11 << 24;

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
