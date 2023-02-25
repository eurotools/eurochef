pub mod gx;
pub mod pc;
// pub mod ps2;
pub mod xbox;

use eurochef_edb::versions::Platform;
use image::RgbaImage;

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
        p => panic!("Unsupported platform for texture decoding: {p:?}"),
    }
}
