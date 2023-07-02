use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use eurochef_edb::versions::Platform;

#[derive(Debug)]
pub struct DissectedFilelistPath {
    /// Root directory, can be drive letter or directory
    pub root: String,
    /// Game ID, eg. 'gforce', 'spyro', 'potccg'
    pub game: String,
    /// 'binary', 'albert', 'sonix' or 'gamespec'
    pub category: String,
    /// Platform directory, eg. '_bin_pc', '_bin_xe'
    pub platform: Platform,
    /// Filename, eg. text.edb
    pub filename: String,
}

impl DissectedFilelistPath {
    pub fn dissect<P: AsRef<Path>>(path: P) -> Option<DissectedFilelistPath> {
        let path = path.as_ref();
        let parts: Vec<&OsStr> = path.iter().collect();
        let filename = path.file_name()?.to_string_lossy().to_string();

        let platform_index = parts
            .iter()
            .position(|p| p.to_string_lossy().to_lowercase().starts_with("_bin_"))?;

        let platform = parts.get(platform_index)?;
        let category = parts.get(platform_index.checked_sub(1)?)?;
        let game = parts.get(platform_index.checked_sub(2)?)?;
        let root: PathBuf = parts.iter().take(platform_index - 2).collect();

        Some(DissectedFilelistPath {
            root: root.to_string_lossy().to_string(),
            game: game.to_string_lossy().to_string(),
            category: category.to_string_lossy().to_string(),
            platform: Platform::from_shorthand(platform.to_string_lossy().get(5..)?)?,
            filename,
        })
    }

    pub fn hashcodes_file(&self) -> PathBuf {
        [&self.root, &self.game, "albert", "hashcodes.h"]
            .iter()
            .collect()
    }

    pub fn sound_hashcodes_file(&self) -> PathBuf {
        [&self.root, &self.game, "sonix", "sound.h"]
            .iter()
            .collect()
    }

    pub fn dir_relative(&self) -> PathBuf {
        [
            &self.game,
            &self.category,
            &format!("_bin_{}", self.platform.shorthand()),
        ]
        .iter()
        .collect()
    }

    pub fn dir_absolute(&self) -> PathBuf {
        [
            &self.root,
            &self.game,
            &self.category,
            &format!("_bin_{}", self.platform.shorthand()),
        ]
        .iter()
        .collect()
    }
}
