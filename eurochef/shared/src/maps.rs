use std::{collections::BTreeMap, mem::transmute};

use eurochef_edb::Hashcode;
use nohash_hasher::IntMap;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Clone)]
pub struct UXGeoTrigger {
    // TODO(cohae): Is this related to a refptr?
    pub link_ref: i32,

    pub ttype: String,
    pub tsubtype: Option<String>,

    pub debug: u16,
    pub game_flags: u32,
    pub trig_flags: u32,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],

    pub data: Vec<Option<u32>>,
    pub links: Vec<i32>,
    pub extra_data: Vec<u32>,
}

fn default_icon_scale() -> f32 {
    0.25
}

fn default_engine_values() -> BTreeMap<u32, TriggerValue> {
    BTreeMap::from([
        (
            0,
            TriggerValue::new(Some("Visual Object"), DefinitionDataType::Hashcode),
        ),
        (
            1,
            TriggerValue::new(Some("File"), DefinitionDataType::Hashcode),
        ),
        (
            2,
            TriggerValue::new(Some("GameScript Index"), DefinitionDataType::U32),
        ),
        (
            3,
            TriggerValue::new(Some("Collision Index"), DefinitionDataType::U32),
        ),
        (
            4,
            TriggerValue::new(Some("Trigger Color"), DefinitionDataType::U32),
        ),
    ])
}

#[derive(Clone, Debug, Deserialize)]
pub struct TriggerInformation {
    #[serde(default = "default_icon_scale")]
    pub icon_scale: f32,
    #[serde(default = "default_engine_values")]
    pub extra_values: BTreeMap<u32, TriggerValue>,
    pub triggers: BTreeMap<u32, TriggerDefinition>,
}

impl Default for TriggerInformation {
    fn default() -> Self {
        Self {
            icon_scale: default_icon_scale(),
            extra_values: default_engine_values(),
            triggers: Default::default(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct TriggerDefinition {
    pub name: String,
    pub icon: Option<String>,
    #[serde(default)]
    pub values: BTreeMap<u32, TriggerValue>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TriggerValue {
    pub name: Option<String>,
    #[serde(alias = "type", default)]
    pub dtype: DefinitionDataType,
}

impl TriggerValue {
    pub fn new(name: Option<&str>, dtype: DefinitionDataType) -> Self {
        Self {
            name: name.map(|v| v.to_owned()),
            dtype,
        }
    }
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DefinitionDataType {
    Unknown32,
    U32,
    Float,
    Hashcode,
}

impl DefinitionDataType {
    pub fn to_string(&self, hashcodes: &IntMap<u32, String>, v: u32) -> String {
        match self {
            DefinitionDataType::Unknown32 => {
                if (v & 0xffff0000) != 0 {
                    if let Some(hc) = hashcodes.get(&v) {
                        return format!("{hc} (0x{:x})", v);
                    }
                }
                format!("{} (0x{:x})", human_num(v), v)
            }
            DefinitionDataType::U32 => {
                if v > 9999 {
                    format!("0x{v:x}")
                } else {
                    format!("{v}")
                }
            }
            DefinitionDataType::Float => unsafe { format!("{:.5}", transmute::<u32, f32>(v)) },
            DefinitionDataType::Hashcode => format_hashcode(hashcodes, v),
        }
    }

    pub fn size(&self) -> usize {
        match self {
            DefinitionDataType::Unknown32 => 4,
            DefinitionDataType::U32 => 4,
            DefinitionDataType::Float => 4,
            DefinitionDataType::Hashcode => 4,
        }
    }
}

pub fn format_hashcode(hashcodes: &IntMap<Hashcode, String>, hc: Hashcode) -> String {
    if hc == Hashcode::MAX {
        return "HT_None".to_string();
    }
    if hc == 0 {
        return "HT_Zero".to_string();
    }

    let hashcode = hashcodes.get(&hc);

    if let Some(hc) = hashcode {
        hc.clone()
    } else {
        let is_local = (hc & 0x80000000) != 0;

        // TODO(cohae): Check if the amount of type/index bits are correct
        if let Some(hc_base) = hashcodes.get(&(hc & 0x7fff0000)) {
            let hc_base_stripped = hc_base
                .strip_suffix("_HASHCODE_BASE")
                .unwrap_or("HT_Invalid");

            if is_local {
                format!("HT_Local_{}_{hc:08x}", &hc_base_stripped[3..])
            } else {
                format!("{hc_base_stripped}_Unknown_{hc:08x}")
            }
        } else {
            if is_local {
                format!("HT_Local_Invalid_{hc:08x}")
            } else {
                format!("HT_Invalid_{hc:08x}")
            }
        }
    }
}

impl Default for DefinitionDataType {
    fn default() -> Self {
        DefinitionDataType::Unknown32
    }
}

// https://github.com/Swyter/poptools/blob/9a22651d7cb16a1edb7894c36e9695138b25b2c1/pop_djinn_sav.bt#L32
fn human_num(v: u32) -> String {
    let i = v as i32;
    let f: f32 = unsafe { transmute(v) };

    if i > -9999 && i < 9999 {
        return i.to_string();
    }
    if f < -0.003 && f > -1e7 {
        return format!("{f:.2}");
    }
    if f > 0.003 && f < 1e7 {
        return format!("{f:.2}");
    }
    return format!("0x{v:x}/{f:.2}");
}
