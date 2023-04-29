use std::io::{Read, Seek, Write};

use eurochef_edb::{
    binrw::{BinReaderExt, Endian},
    header::EXGeoHeader,
    text::{EXGeoSpreadSheet, EXGeoTextItem},
};

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
    pub fn read_all<R: Read + Seek>(
        header: EXGeoHeader,
        reader: &mut R,
        endian: Endian, // TODO: Shouldn't need to pass this for every function
    ) -> Vec<Self> {
        let mut spreadsheets = vec![];

        for s in &header.spreadsheet_list {
            if s.m_type != 1 {
                continue;
            }

            let mut spreadsheet = UXGeoSpreadsheet(vec![]);
            reader
                .seek(std::io::SeekFrom::Start(s.common.address as u64))
                .unwrap();

            let sheader = reader
                .read_type::<EXGeoSpreadSheet>(endian)
                .expect("Failed to read spreadsheet");

            for s in sheader.sections {
                let mut section = UXGeoTextSection {
                    hashcode: s.hashcode,
                    entries: vec![],
                };
                let refpointer = &header.refpointer_list.data[s.refpointer_index as usize];

                reader
                    .seek(std::io::SeekFrom::Start(refpointer.address as u64))
                    .unwrap();

                reader.seek(std::io::SeekFrom::Current(4)).unwrap(); // Skip commonobject
                let text_count = reader.read_type::<u32>(endian).unwrap();
                for _i in 0..text_count {
                    let item = reader
                        .read_type::<EXGeoTextItem>(endian)
                        .expect("Failed to read textitem");
                    section.entries.push(UXGeoTextItem {
                        hashcode: item.hashcode,
                        text: item.string.data.to_string(),
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
