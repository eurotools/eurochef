use std::io::{Seek, Write};

use eurochef_edb::{
    binrw::BinReaderExt,
    text::{EXGeoSpreadSheet, EXGeoTextItem},
};
use tracing::warn;

use crate::edb::EdbFile;

#[derive(Clone)]
pub struct UXGeoSpreadsheet(pub Vec<UXGeoTextSection>);

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
    pub fn read_all(edb: &mut EdbFile) -> Vec<Self> {
        let mut spreadsheets = vec![];

        let header = edb.header.clone();
        for s in &header.spreadsheet_list {
            if s.stype != 1 {
                warn!("Skipping data spreadsheet");
                continue;
            }

            let mut spreadsheet = UXGeoSpreadsheet(vec![]);
            edb.seek(std::io::SeekFrom::Start(s.common.address as u64))
                .unwrap();

            let sheader = edb
                .read_type::<EXGeoSpreadSheet>(edb.endian)
                .expect("Failed to read spreadsheet");

            for s in sheader.sections {
                let mut section = UXGeoTextSection {
                    hashcode: s.hashcode,
                    entries: vec![],
                };
                let refpointer = &edb.header.refpointer_list[s.refpointer_index as usize];

                edb.seek(std::io::SeekFrom::Start(refpointer.address as u64))
                    .unwrap();

                edb.seek(std::io::SeekFrom::Current(4)).unwrap(); // Skip commonobject
                let text_count = edb.read_type::<u32>(edb.endian).unwrap();
                for _i in 0..text_count {
                    let item = edb
                        .read_type::<EXGeoTextItem>(edb.endian)
                        .expect("Failed to read textitem");
                    section.entries.push(UXGeoTextItem {
                        hashcode: item.hashcode,
                        text: item.string.to_string(),
                        sound_hashcode: item.sound_hashcode,
                    });
                }

                spreadsheet.0.push(section);
            }

            spreadsheets.push(spreadsheet);
        }

        spreadsheets
    }

    pub fn export_to_csv<W: Write>(
        &self,
        writer: &mut W,
        section_hashcode: u32,
    ) -> anyhow::Result<()> {
        let section = self
            .0
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
