use anyhow::Context;
use nohash_hasher::IntMap;
use serde::Deserialize;
use std::io::{Seek, SeekFrom, Write};

use crate::maps::DefinitionDataType;
use eurochef_edb::common::EXRelPtr;
use eurochef_edb::{
    binrw::BinReaderExt,
    edb::EdbFile,
    text::{EXGeoSpreadSheet, EXGeoTextItem},
    Hashcode,
};
use tracing::warn;

#[derive(Clone)]
pub enum UXGeoSpreadsheet {
    Data(Vec<UXGeoDataSheet>),
    Text(Vec<UXGeoTextSection>),
}

#[derive(Clone)]
pub struct UXGeoDataSheet {
    pub row_count: u32,
    // We can't find the file size from here, so we just supply an offset
    /// Absolute location of the row data
    pub address: u32,
}

#[derive(Clone)]
pub struct UXGeoTextSection {
    pub hashcode: u32,
    pub entries: Vec<UXGeoTextItem>,
}

#[derive(Clone)]
pub struct UXGeoTextItem {
    pub hashcode: u32,
    pub text: String,
    pub sound_hashcode: u32,
    // pub userdata: EXRelPtr,
}

impl UXGeoSpreadsheet {
    pub fn read_all(edb: &mut EdbFile) -> anyhow::Result<Vec<(Hashcode, Self)>> {
        let mut spreadsheets = vec![];

        let header = edb.header.clone();
        for s in &header.spreadsheet_list {
            let spreadsheet = match s.stype {
                1 => {
                    let mut spreadsheet = vec![];
                    edb.seek(SeekFrom::Start(s.common.address as u64)).unwrap();

                    let sheader = edb
                        .read_type::<EXGeoSpreadSheet>(edb.endian)
                        .context("Failed to read spreadsheet")?;

                    for s in sheader.sections {
                        let mut section = UXGeoTextSection {
                            hashcode: s.hashcode,
                            entries: vec![],
                        };
                        let refpointer = &edb.header.refpointer_list[s.refpointer_index as usize];

                        edb.seek(SeekFrom::Start(refpointer.address as u64))
                            .unwrap();

                        edb.seek(SeekFrom::Current(4)).unwrap(); // Skip commonobject
                        let text_count = edb.read_type::<u32>(edb.endian).unwrap();
                        for _i in 0..text_count {
                            let item = edb
                                .read_type::<EXGeoTextItem>(edb.endian)
                                .context("Failed to read textitem")?;
                            section.entries.push(UXGeoTextItem {
                                hashcode: item.hashcode,
                                text: item.string.to_string(),
                                sound_hashcode: item.sound_hashcode,
                            });
                        }

                        spreadsheet.push(section);
                    }

                    UXGeoSpreadsheet::Text(spreadsheet)
                }
                2 => {
                    let mut spreadsheet = vec![];
                    edb.seek(SeekFrom::Start(s.common.address as u64)).unwrap();

                    let sheet_count: u32 = edb.read_type(edb.endian)?;
                    for _ in 0..sheet_count {
                        let ptr: EXRelPtr = edb.read_type(edb.endian)?;
                        let save_pos = edb.stream_position()?;
                        edb.seek(SeekFrom::Start(ptr.offset_absolute()))?;
                        let sheet = UXGeoDataSheet {
                            row_count: edb.read_type(edb.endian)?,
                            address: edb.stream_position()? as u32,
                        };
                        edb.seek(SeekFrom::Start(save_pos))?;

                        spreadsheet.push(sheet);
                    }

                    UXGeoSpreadsheet::Data(spreadsheet)
                }
                u => {
                    warn!("Unknown spreadsheet type {} (0x{:x})", u, s.common.address);
                    continue;
                }
            };

            spreadsheets.push((s.common.hashcode, spreadsheet));
        }

        Ok(spreadsheets)
    }

    pub fn export_text_to_csv<W: Write>(
        &self,
        writer: &mut W,
        section_hashcode: u32,
    ) -> anyhow::Result<()> {
        let spreadsheet = match self {
            UXGeoSpreadsheet::Text(v) => v,
            _ => anyhow::bail!("Spreadsheet is not a text spreadsheet"),
        };

        let section = spreadsheet
            .iter()
            .find(|s| s.hashcode == section_hashcode)
            .ok_or(anyhow::anyhow!("Failed to find section"))?;

        writeln!(writer, "section,hashcode,sound_hashcode,text")?;
        for item in &section.entries {
            writeln!(
                writer,
                "{:08x},{:08x},{:08x},\"{}\"",
                section.hashcode,
                item.hashcode,
                item.sound_hashcode,
                item.text.replace('"', "\"\"").replace('\n', "\\n")
            )?;
        }

        Ok(())
    }
}

pub type SpreadsheetDefinitions = IntMap<Hashcode, SpreadsheetFileDefinition>;

/// Represents all spreadsheets in a file
#[derive(Clone, Default, Debug, Deserialize)]
pub struct SpreadsheetFileDefinition(pub IntMap<Hashcode, SpreadsheetDefinition>);

/// Represents all sheets in a spreadsheet
#[derive(Clone, Default, Debug, Deserialize)]
pub struct SpreadsheetDefinition(pub IntMap<usize, DataSheetDefinition>);

#[derive(Clone, Debug, Deserialize)]
pub struct DataSheetDefinition {
    pub row_size: usize,
    #[serde(default)]
    pub columns: Vec<DataSheetColumn>,
}

#[derive(Clone, Debug, Deserialize)]

pub struct DataSheetColumn {
    pub name: Option<String>,
    #[serde(alias = "type", default)]
    pub dtype: DefinitionDataType,
}
