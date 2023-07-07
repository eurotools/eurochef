use crate::filesystem::path::DissectedFilelistPath;
use crate::hashcodes::parse_hashcodes;
use eurochef_edb::Hashcode;
use nohash_hasher::IntMap;
use std::path::PathBuf;
use tracing::warn;

pub mod path;

pub fn load_hashcodes(path: &DissectedFilelistPath, load_sonix: bool) -> IntMap<Hashcode, String> {
    let mut hashcodes = IntMap::default();
    if let Ok(hfs) = std::fs::read_to_string(path.hashcodes_file()) {
        hashcodes.extend(parse_hashcodes(&hfs));
    } else {
        // Fall back to the 'hashcodes' directory
        let exe_path = std::env::current_exe().unwrap();
        let exe_dir = exe_path.parent().unwrap();
        if let Ok(hfs) = std::fs::read_to_string(exe_dir.join(PathBuf::from_iter(&[
            "hashcodes",
            &path.game,
            "albert",
            "hashcodes.h",
        ]))) {
            hashcodes.extend(parse_hashcodes(&hfs));
        } else {
            warn!("Couldn't find a hashcodes.h file for {} :(", path.game);
        }
    }

    if load_sonix {
        if let Ok(hfs) = std::fs::read_to_string(path.sound_hashcodes_file()) {
            hashcodes.extend(parse_hashcodes(&hfs));
        } else {
            // Fall back to the 'hashcodes' directory
            let exe_path = std::env::current_exe().unwrap();
            let exe_dir = exe_path.parent().unwrap();
            if let Ok(hfs) = std::fs::read_to_string(exe_dir.join(PathBuf::from_iter(&[
                "hashcodes",
                &path.game,
                "sonix",
                "sound.h",
            ]))) {
                hashcodes.extend(parse_hashcodes(&hfs));
            } else {
                warn!("Couldn't find a sound.h file for {} :(", path.game);
            }
        }
    }

    hashcodes
}
