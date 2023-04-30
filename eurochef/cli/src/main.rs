#[macro_use]
extern crate tracing;

mod edb;
mod filelist;

use anyhow::Context;
use clap::{Parser, Subcommand};
use clap_num::maybe_hex;
use eurochef_edb::versions::Platform;

#[derive(clap::ValueEnum, PartialEq, Debug, Clone)]
pub enum PlatformArg {
    Pc,
    Xb,
    Xbox,
    Xbox360,
    Ps2,
    Ps3,
    Gc,
    Gamecube,
    Wii,
    WiiU,
}

impl Into<Platform> for PlatformArg {
    fn into(self) -> Platform {
        match self {
            PlatformArg::Pc => Platform::Pc,
            PlatformArg::Xbox | PlatformArg::Xb => Platform::Xbox,
            PlatformArg::Xbox360 => Platform::Xbox360,
            PlatformArg::Ps2 => Platform::Ps2,
            PlatformArg::Ps3 => Platform::Ps3,
            PlatformArg::Gamecube | PlatformArg::Gc => Platform::GameCube,
            PlatformArg::Wii => Platform::Wii,
            PlatformArg::WiiU => Platform::WiiU,
        }
    }
}

#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Commands for working with filelists
    Filelist {
        #[command(subcommand)]
        subcommand: FilelistCommand,
    },
    Edb {
        #[command(subcommand)]
        subcommand: EdbCommand,
    },
}

#[derive(Subcommand, Debug, Clone)]
enum EdbCommand {
    /// Extract entities
    Entities {
        /// .edb file to read
        filename: String,

        /// Output folder for textures (default: "./entities/{filename}/")
        output_folder: Option<String>,

        /// Override for platform detection
        #[arg(value_enum, short, long, ignore_case = true)]
        platform: Option<PlatformArg>,

        /// Don't embed textures into the output file
        #[arg(short = 'e', long)]
        no_embed: bool,

        /// Remove transparent surfaces
        #[arg(short = 't', long)]
        no_transparent: bool,
    },
    /// Extract spreadsheets
    Spreadsheets {
        /// .edb file to read
        filename: String,

        /// Section ID to read
        #[arg(value_parser = maybe_hex::<u32>)]
        section: Option<u32>,
    },
    // /// Extract maps
    // Maps {
    //     /// .edb file to read
    //     filename: String,

    //     /// Override for platform detection
    //     #[arg(value_enum, short, long, ignore_case = true)]
    //     platform: Option<PlatformArg>,
    // },
    /// Extract textures
    Textures {
        /// .edb file to read
        filename: String,

        /// Output folder for textures (default: "./textures/{filename}/")
        output_folder: Option<String>,

        /// Override for platform detection
        #[arg(value_enum, short, long, ignore_case = true)]
        platform: Option<PlatformArg>,

        /// Output file format to use (supported: tga, png, qoi)
        #[arg(short, long, default_value("tga"))]
        format: String,
    },
}

#[derive(Subcommand, Debug, Clone)]
enum FilelistCommand {
    /// Extract a filelist
    Extract {
        /// .bin file to use
        filename: String,

        /// The folder to extract to (will be created if it doesnt exist)
        #[arg(default_value = "./")]
        output_folder: String,

        /// Create a .scr file in the output folder
        #[arg(short = 's', long)]
        create_scr: bool,
    },
    /// Create a new filelist from a folder
    Create {
        /// Folder to read files from
        input_folder: String,

        /// Destination for the generated filelist (without filename extension)
        #[arg(default_value = "./Filelist")]
        output_file: String,

        #[arg(long, short = 'l', default_value_t = 'x')]
        drive_letter: char,

        /// Supported versions: 5, 6, 7
        #[arg(long, short, default_value_t = 7)]
        version: u32,

        #[arg(value_enum, short, long, ignore_case = true)]
        platform: PlatformArg,

        /// Maximum size per data file, might be overridden by a .scr file
        #[arg(long, short = 'z', default_value_t = 0x80000000, value_parser = maybe_hex::<u32>)]
        split_size: u32,

        /// .scr file to read options from (currently doesnt support wildcards)
        #[arg(long, short)]
        scr_file: Option<String>,
    },
}

pub fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match &args.cmd {
        Command::Filelist { subcommand } => handle_filelist(subcommand.clone()),
        Command::Edb { subcommand } => handle_edb(subcommand.clone()),
    }
}

fn handle_edb(cmd: EdbCommand) -> anyhow::Result<()> {
    match cmd {
        EdbCommand::Entities {
            filename,
            output_folder,
            platform,
            no_embed,
            no_transparent,
        } => edb::entities::execute_command(
            filename,
            platform,
            output_folder,
            no_embed,
            no_transparent,
        ),
        // EdbCommand::Maps { filename, platform } => edb::maps::execute_command(filename, platform),
        EdbCommand::Spreadsheets { filename, section } => {
            edb::spreadsheets::execute_command(filename, section)
        }
        EdbCommand::Textures {
            filename,
            platform,
            output_folder,
            format,
        } => edb::textures::execute_command(filename, platform, output_folder, format),
    }
}

fn handle_filelist(cmd: FilelistCommand) -> anyhow::Result<()> {
    match cmd {
        FilelistCommand::Extract {
            filename,
            output_folder,
            create_scr,
        } => filelist::extract::execute_command(filename, output_folder, create_scr)
            .context("Failed to extract filelist"),
        FilelistCommand::Create {
            input_folder,
            output_file,
            drive_letter,
            version,
            platform,
            split_size,
            scr_file,
        } => filelist::create::execute_command(
            input_folder,
            output_file,
            drive_letter,
            version,
            platform,
            split_size,
            scr_file,
        )
        .context("Failed to create filelist"),
    }
}
