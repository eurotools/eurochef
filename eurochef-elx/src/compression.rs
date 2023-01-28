use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub enum CompressionMethod {
    Polynomial,
    Wavelet,
}
