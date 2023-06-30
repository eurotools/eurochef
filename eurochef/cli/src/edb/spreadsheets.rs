use std::{fs::File, io::BufReader};

use eurochef_edb::{edb::EdbFile, versions::Platform};
use eurochef_shared::spreadsheets::UXGeoSpreadsheet;

pub fn execute_command(filename: String, section: Option<u32>) -> anyhow::Result<()> {
    let mut file = File::open(filename)?;
    let mut reader = BufReader::new(&mut file);
    let mut edb = EdbFile::new(&mut reader, Platform::Pc)?;

    let spreadsheets = UXGeoSpreadsheet::read_all(&mut edb);
    assert!(spreadsheets.len() <= 1);
    if spreadsheets.is_empty() {
        println!("No spreadsheets found in file");
        return Ok(());
    }
    let spreadsheet = &spreadsheets[0];

    if let Some(section) = section {
        if spreadsheet
            .0
            .iter()
            .find(|s| s.hashcode == section)
            .is_none()
        {
            println!(
                "Section {:08x} could not be found. Available sections: {}",
                section,
                spreadsheet
                    .0
                    .iter()
                    .map(|s| format!("{:08x}", s.hashcode))
                    .collect::<Vec<String>>()
                    .join(", ")
            )
        }

        spreadsheet.export_to_csv(&mut std::io::stdout(), section)?;
    } else {
        for s in &spreadsheet.0 {
            println!("# Section {:08x}", s.hashcode);
            spreadsheet.export_to_csv(&mut std::io::stdout(), s.hashcode)?;
        }
    }

    info!("Successfully extracted spreadsheets!");

    Ok(())
}
