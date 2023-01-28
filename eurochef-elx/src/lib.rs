pub mod compression;

pub use quick_xml;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct ELXML {
    #[serde(rename = "@type")]
    pub filetype: String,

    #[serde(rename = "@version")]
    pub version: String,

    pub header: Header,
    pub asset: Asset,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Header {
    pub a_dependency_ids: String,
    pub h_dependency_flags: String,
    pub b_dependency_version_count: String,
    pub c_dependency_version_data: String,
    pub d_sound_ids: String,
    pub e_autoinclude_in_resources: bool,
    pub j_uid: String,
    pub f_subfile_uid: String,
    pub g_resource_group_overides: String,

    /// Base64-encoded BGR image data
    /// Prefixed with 3 u16s, the first unknown (format?), second width and third height
    pub i_thumbnail: Option<String>,

    pub k_cat_user: String,
    pub l_cat_engine: String,
    pub m_has_collisions: bool,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Asset {
    pub auto_include_in_resources: bool,
    pub save_count: u32,
    pub lastsavedwith: String,
    pub lastsavedby: String,
    pub lastsavedat: String,
    pub fps: String,
    // FIXME: quick_xml handles enums like <enum/>, not <node>enum<node/>
    // can we fix this nicely?
    // pub comp_method: CompressionMethod,
    pub comp_method: String,
    pub comp_tol: String,
    pub comp_type: String,
    pub cache_helper: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct BinaryData {
    #[serde(rename = "@datatype")]
    pub datatype: String,
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@size")]
    pub size: u32,

    #[serde(rename = "block")]
    pub blocks: Vec<DataBlock>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DataBlock {
    #[serde(rename = "@size")]
    pub size: u32,

    /// Data is encoded as base64
    #[serde(rename = "$text")]
    pub data: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Mesh {
    #[serde(rename = "@alphasorting_bias")]
    pub alphasorting_bias: String,
    #[serde(rename = "@alphasorting_type")]
    pub alphasorting_type: String,
    #[serde(rename = "@can_cast_shadows")]
    pub can_cast_shadows: bool,
    #[serde(rename = "@can_receive_shadows")]
    pub can_receive_shadows: bool,
    #[serde(rename = "@layer_name")]
    pub layer_name: String,
    #[serde(rename = "@name")]
    pub name: String,

    pub polygons: Polygons,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Polygons {
    #[serde(rename = "@count")]
    pub count: u32,
    #[serde(rename = "@renderset")]
    pub renderset: String,

    /// Space-separated indices, 3 per element
    pub p: Vec<String>,
}
