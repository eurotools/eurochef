use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::process::exit;
use std::{fs::File, io::BufReader};

use eurochef_edb::binrw::BinReaderExt;
use eurochef_edb::{edb::EdbFile, versions::Platform, Hashcode};
use eurochef_shared::filesystem::path::DissectedFilelistPath;
use eurochef_shared::maps::{format_hashcode, DefinitionDataType};
use eurochef_shared::spreadsheets::{SpreadsheetDefinitions, UXGeoSpreadsheet};

pub fn execute_command(filename: String, output_folder: Option<String>) -> anyhow::Result<()> {
    let output_folder = output_folder.unwrap_or(format!(
        "./spreadsheets/{}/",
        Path::new(&filename)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string(),
    ));
    let output_folder = Path::new(&output_folder);
    std::fs::create_dir_all(output_folder)?;

    let file = File::open(&filename)?;
    let reader = BufReader::new(file);
    let mut edb = EdbFile::new(Box::new(reader), Platform::Pc)?;

    let (spreadsheet_definitions, hashcodes) =
        if let Some(dissected_path) = DissectedFilelistPath::dissect(&filename) {
            let exe_path = std::env::current_exe().unwrap();
            let exe_dir = exe_path.parent().unwrap();
            let v = std::fs::read_to_string(exe_dir.join(&format!(
                "./assets/spreadsheets_{}.yml",
                dissected_path.game
            )))
            .unwrap_or_default();

            let spreadsheet_definitions: SpreadsheetDefinitions = match serde_yaml::from_str(&v) {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to parse spreadsheet definitions: {e}");
                    Default::default()
                }
            };

            (
                spreadsheet_definitions,
                eurochef_shared::filesystem::load_hashcodes(&dissected_path, true),
            )
        } else {
            error!("Given path is not a valid EngineX-compatible path");
            (Default::default(), Default::default())
        };

    let spreadsheet_definition = spreadsheet_definitions
        .get(&edb.header.hashcode)
        .cloned()
        .unwrap_or_default();

    for (file_hashcode, sfile) in &spreadsheet_definitions {
        for (hashcode, spreadsheet) in &sfile.0 {
            for (_sheet_num, sheet) in &spreadsheet.0 {
                if sheet.columns.is_empty() {
                    continue;
                }

                let total_column_size: usize = sheet.columns.iter().map(|v| v.dtype.size()).sum();
                if total_column_size != sheet.row_size {
                    error!("Spreadsheet {hashcode:08x} (file {file_hashcode:08x}) has an invalid row size (total row size {}, row_size {})", total_column_size, sheet.row_size);
                    exit(-1);
                }
            }
        }
    }

    let spreadsheets = UXGeoSpreadsheet::read_all(&mut edb)?;
    if spreadsheets.is_empty() {
        println!("No spreadsheets found in file");
        return Ok(());
    }

    for (hashcode, spreadsheet) in &spreadsheets {
        info!(
            "Extracting spreadsheet {hashcode:08x} ({} sheets)",
            match &spreadsheet {
                UXGeoSpreadsheet::Data(v) => v.len(),
                UXGeoSpreadsheet::Text(v) => v.len(),
            }
        );

        let mut output = File::create(output_folder.join(format!("{hashcode:08x}.csv")))?;

        match spreadsheet {
            UXGeoSpreadsheet::Data(data) => {
                for (sheet_num, sheet) in data.iter().enumerate() {
                    edb.seek(SeekFrom::Start(sheet.address as u64))?;
                    let sheet_definition = match spreadsheet_definition.0.get(&hashcode) {
                        None => {
                            error!("Missing spreadsheet definition for file {:08x} spreadsheet {hashcode:08x} sheet #{sheet_num} (address 0x{:x})", edb.header.hashcode, sheet.address);
                            continue;
                        }
                        Some(s) => match s.0.get(&sheet_num) {
                            None => {
                                error!("Missing sheet definition for file {:08x} spreadsheet {hashcode:08x} sheet #{sheet_num} (address 0x{:x})", edb.header.hashcode, sheet.address);
                                continue;
                            }
                            Some(s) => s,
                        },
                    };

                    if sheet_definition.columns.is_empty() {
                        warn!("Missing column definitions for file {:08x} spreadsheet {hashcode:08x} sheet #{sheet_num} (address 0x{:x})", edb.header.hashcode, sheet.address);
                        writeln!(output, "data")?;
                        let mut row_data = vec![0u8; sheet_definition.row_size];
                        for _ in 0..sheet.row_count {
                            edb.read_exact(&mut row_data)?;
                            writeln!(output, "{}", hex::encode(&row_data))?;
                        }
                    } else {
                        let header = sheet_definition
                            .columns
                            .iter()
                            .enumerate()
                            .map(|(i, c)| c.name.clone().unwrap_or(format!("row_{i}")))
                            .collect::<Vec<String>>()
                            .join(",");
                        writeln!(output, "{}", header)?;

                        for _ in 0..sheet.row_count {
                            let mut row = vec![];

                            for c in &sheet_definition.columns {
                                match c.dtype {
                                    DefinitionDataType::Unknown32 => {
                                        let v: u32 = edb.read_type(edb.endian)?;
                                        row.push(format!("0x{v:x}"));
                                    }
                                    DefinitionDataType::U32 => {
                                        let v: u32 = edb.read_type(edb.endian)?;
                                        row.push(v.to_string());
                                    }
                                    DefinitionDataType::Float => {
                                        let v: f32 = edb.read_type(edb.endian)?;
                                        row.push(v.to_string());
                                    }
                                    DefinitionDataType::Hashcode => {
                                        let v: Hashcode = edb.read_type(edb.endian)?;
                                        row.push(format_hashcode(&hashcodes, v));
                                    }
                                }
                            }

                            writeln!(output, "{}", row.join(","))?;
                        }
                    }
                }
            }

            UXGeoSpreadsheet::Text(text) => {
                for s in text {
                    writeln!(output, "# Section {:08x}", s.hashcode)?;
                    spreadsheet.export_text_to_csv(&mut output, s.hashcode)?;
                }
            }
        }
    }

    info!("Successfully extracted spreadsheets!");

    Ok(())
}
