use thiserror::Error;

pub type Result<T> = std::result::Result<T, EurochefError>;

#[derive(Error, Debug)]
pub enum EurochefError {
    #[error("Unsupported: {0}")]
    Unsupported(UnsupportedError),

    #[error("Input/output error: {0}")]
    Io(#[from] std::io::Error),

    #[error("BinRW error")]
    BinRw(#[from] binrw::Error),

    #[error("Error")]
    Misc(#[from] anyhow::Error),
}

#[derive(Error, Debug)]
pub enum UnsupportedError {
    #[error(
        "The specified file is built for an unsupported version of EngineX (version code 0x{0:x})"
    )]
    Version(u32),

    #[error("The specified file is built for EngineXT (version code 0x{0:x})")]
    EngineXT(u32),
}
