use std::{
    io::{Cursor, Seek},
    sync::{Arc, Mutex},
};

use eurochef_edb::{
    binrw::{BinReaderExt, Endian},
    common::EXVector,
    script::EXGeoAnimScript,
};
use eurochef_shared::maps::format_hashcode;
use glam::{Quat, Vec3};
use nohash_hasher::IntMap;

use crate::entity_frame::RenderableTexture;

use super::{entity::EntityRenderer, viewer::RenderContext};

pub struct ScriptRenderer {
    endian: Endian,
    script: EXGeoAnimScript,
    entities: Vec<(u32, Arc<Mutex<EntityRenderer>>)>,
}

impl ScriptRenderer {
    pub fn new(
        endian: Endian,
        script: EXGeoAnimScript,
        entities: Vec<(u32, Arc<Mutex<EntityRenderer>>)>,
        hashcodes: Arc<IntMap<u32, String>>,
    ) -> Self {
        /// Hexadecimal string, separated every 4 bytes
        fn hex_split(data: &[u8]) -> String {
            hex_string(data)
                .chars()
                .collect::<Vec<char>>()
                .chunks(8)
                .map(|chunk| chunk.iter().collect())
                .collect::<Vec<String>>()
                .join(" ")
        }

        fn hex_string(data: &[u8]) -> String {
            data.iter()
                .map(|b| format!("{b:02x}").chars().collect::<Vec<char>>())
                .flatten()
                .collect()
        }
        println!(
            "Script ({:.1} fps, {} threads, length {})",
            script.frame_rate, script.thread_count, script.length
        );
        for cmd in &script.commands {
            let mut cmd_cur = Cursor::new(&cmd.data);
            if cmd.cmd != 0x12 {
                print!(
                    "\tstart={} length={} t={} pt{} u0={} u1={} ",
                    cmd.start, cmd.length, cmd.thread, cmd.parent_thread, cmd.unk0, cmd.unk1
                );
            } else {
                print!("\t");
            }

            match cmd.cmd - 1 {
                1 => {
                    cmd_cur.seek(std::io::SeekFrom::End(-16)).unwrap();
                    let (file_hash, anim_hash) = cmd_cur.read_type::<(u32, u32)>(endian).unwrap();
                    println!(
                        "- Animation {} (file {}) [{}]",
                        format_hashcode(&hashcodes, anim_hash),
                        hashcodes.get(&file_hash).unwrap_or(&"[Local]".to_string()),
                        hex_split(&cmd.data)
                    );
                }
                2 => {
                    cmd_cur.seek(std::io::SeekFrom::End(-8)).unwrap();
                    let (file_hash, ent_hash) = cmd_cur.read_type::<(u32, u32)>(endian).unwrap();
                    println!(
                        "- Entity {} (file {}) [{}]",
                        format_hashcode(&hashcodes, ent_hash),
                        hashcodes.get(&file_hash).unwrap_or(&"[Local]".to_string()),
                        hex_split(&cmd.data)
                    );
                }
                3 => {
                    cmd_cur.seek(std::io::SeekFrom::End(-8)).unwrap();
                    let (file_hash, ent_hash) = cmd_cur.read_type::<(u32, u32)>(endian).unwrap();
                    println!(
                        "- Subscript {} (file {}) [{}]",
                        format_hashcode(&hashcodes, ent_hash),
                        hashcodes.get(&file_hash).unwrap_or(&"[Local]".to_string()),
                        hex_split(&cmd.data)
                    );
                }
                4 => {
                    cmd_cur.seek(std::io::SeekFrom::End(-4)).unwrap();
                    let hashcode = cmd_cur.read_type::<u32>(endian).unwrap();
                    println!(
                        "- Sound {} [{}]",
                        format_hashcode(&hashcodes, hashcode),
                        hex_split(&cmd.data)
                    );
                }
                5 => {
                    cmd_cur.seek(std::io::SeekFrom::End(-4)).unwrap();
                    let hashcode = cmd_cur.read_type::<u32>(endian).unwrap();
                    println!(
                        "- Particle {} [{}]",
                        format_hashcode(&hashcodes, hashcode),
                        hex_split(&cmd.data)
                    );
                }
                9 => {
                    cmd_cur.seek(std::io::SeekFrom::End(-16)).unwrap();
                    let unk = cmd_cur.read_type::<EXVector>(endian).unwrap();
                    println!("- Unk9 {:?} [{}]", unk, hex_split(&cmd.data));
                }
                10 => {
                    let hashcode = cmd_cur.read_type::<u32>(endian).unwrap();
                    println!(
                        "- ScriptEvent {} [{}]",
                        hashcodes
                            .get(&hashcode)
                            .unwrap_or(&format!("{hashcode:08x}")),
                        hex_split(&cmd.data)
                    );
                }
                0x11 => {
                    println!("- Script end");
                }
                u => println!("- Unknown command 0x{:x} [{}]", u, hex_split(&cmd.data)),
            }
        }
        println!();

        Self {
            endian,
            script,
            entities,
        }
    }

    pub fn render(
        &self,
        gl: &glow::Context,
        context: &RenderContext,
        position: Vec3,
        rotation: Quat,
        textures: &[RenderableTexture],
        draw_func: unsafe fn(
            &EntityRenderer,
            &glow::Context,
            &crate::render::viewer::RenderContext<'_>,
            Vec3,
            Quat,
            Vec3,
            f64,
            &[RenderableTexture],
        ),
    ) {
        for c in self
            .script
            .commands
            .iter()
            .filter(|c| c.start == 0 && c.cmd == 3)
        {
            let hashcode = match self.endian {
                Endian::Big => u32::from_be_bytes,
                Endian::Little => u32::from_le_bytes,
            }(c.data[8..12].try_into().unwrap());

            if (hashcode & 0x80000000) != 0 {
                if let Some((_, r)) = self.entities.get((hashcode & 0x0000ffff) as usize) {
                    if let Ok(r) = r.try_lock() {
                        unsafe {
                            draw_func(
                                &r,
                                gl,
                                context,
                                position,
                                rotation,
                                Vec3::ONE,
                                context.uniforms.time as f64,
                                textures,
                            );
                        }
                    }
                }
            }
        }
    }
}
