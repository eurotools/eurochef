#![feature(downcast_unchecked)]

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
pub mod versions;

// Re-export binrw
pub use binrw;

pub type Hashcode = u32;
