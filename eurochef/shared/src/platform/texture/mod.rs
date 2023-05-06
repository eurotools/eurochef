pub mod gx;
pub mod pc;
pub mod xbox;
pub mod xenon;
// pub mod ps2;

use eurochef_edb::versions::Platform;
use image::RgbaImage;

// TODO: Extract to a crate and add better errors
pub trait TextureDecoder {
    /// Returns `None` if the format is invalid
    fn get_data_size(
        &self,
        width: u32,
        height: u32,
        depth: u32,
        format: u8,
    ) -> anyhow::Result<usize>;

    /// Output buffer must be width*height*depth*4 bytes long (RGBA)
    fn decode(
        &self,
        input: &[u8],
        output: &mut RgbaImage,
        width: u32,
        height: u32,
        depth: u32,
        format: u8,
    ) -> anyhow::Result<()>;
}

pub fn create_for_platform(platform: Platform) -> Box<dyn TextureDecoder> {
    match platform {
        Platform::Pc => Box::new(pc::PcTextureDecoder),
        // Platform::Ps2 => Box::new(ps2::Ps2TextureDecoder),
        Platform::GameCube | Platform::Wii => Box::new(gx::GxTextureDecoder),
        Platform::Xbox => Box::new(xbox::XboxTextureDecoder),
        Platform::Xbox360 => Box::new(xenon::XenonTextureDecoder),
        p => panic!("Unsupported platform for texture decoding: {p:?}"),
    }
}
