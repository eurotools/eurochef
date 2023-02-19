use std::{fs::File, io::Seek};

use eurochef_edb::{
    binrw::{BinReaderExt, Endian},
    header::EXGeoHeader,
    text::{EXGeoSpreadSheet, EXGeoTextItem},
};

pub fn execute_command(filename: String) -> anyhow::Result<()> {
    let mut file = File::open(filename)?;
    let endian = if file.read_ne::<u8>().unwrap() == 0x47 {
        Endian::Big
    } else {
        Endian::Little
    };
    file.seek(std::io::SeekFrom::Start(0))?;

    let header = file
        .read_type::<EXGeoHeader>(endian)
        .expect("Failed to read header");
    println!("Read header: {header:#?}");

    for s in &header.spreadsheet_list {
        if s.m_type != 1 {
            continue;
        }

        file.seek(std::io::SeekFrom::Start(s.common.address as u64))?;
        let spreadsheet = file
            .read_type::<EXGeoSpreadSheet>(endian)
            .expect("Failed to read spreadsheet");

        for s in spreadsheet.sections {
            let refpointer = &header.refpointer_list.data[s.refpointer_index as usize];

            file.seek(std::io::SeekFrom::Start(refpointer.address as u64))?;

            // Header format is slightly larger for Spyro
            let text_count = if [213, 236, 221, 240].contains(&header.version) {
                file.seek(std::io::SeekFrom::Current(20))?;
                file.read_type::<u32>(endian).unwrap()
            } else {
                file.seek(std::io::SeekFrom::Current(4))?; // Skip commonobject
                file.read_type::<u32>(endian).unwrap()
            };
            println!("{} strings @ 0x{:x}", text_count, refpointer.address);
            for i in 0..text_count {
                let item = file
                    .read_type::<EXGeoTextItem>(endian)
                    .expect("Failed to read textitem");

                print!("{:08x} - {}", item.hashcode, item.string.data);
                if item.sound_hashcode != 0xffffffff {
                    print!(" (sound hash {:08x})", item.sound_hashcode);
                }

                println!();
            }
            println!()
        }
    }

    Ok(())
}
