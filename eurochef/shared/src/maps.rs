use std::{collections::BTreeMap, mem::transmute};

use nohash_hasher::IntMap;
use serde::{Deserialize, Serialize};
use tracing::warn;

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

    pub raw_data: Vec<u32>,
    pub data: Vec<Option<u32>>,
    pub links: Vec<i32>,
    pub extra_data: Vec<u32>,
}

// TODO(cohae): Move this to eurochef-edb so we can parse trigger collisions
pub fn parse_trigger_data(
    _version: u32,
    trig_flags: u32,
    raw_data: &[u32],
) -> (Vec<Option<u32>>, Vec<i32>, Vec<u32>) {
    let mut data = vec![];
    let mut links = vec![];

    let mut flag_accessor = 1;
    let mut data_offset = 0;

    // TODO(cohae): Some older games use only 8 values instead of 16
    for _ in 0..16 {
        if (trig_flags & flag_accessor) != 0 {
            data.push(Some(raw_data[data_offset]));
            data_offset += 1;
        } else {
            data.push(None);
        }

        flag_accessor <<= 1;
    }

    for _ in 0..8 {
        if (trig_flags & flag_accessor) != 0 {
            links.push(raw_data[data_offset] as i32);
            data_offset += 1;
        } else {
            links.push(-1);
        }

        flag_accessor <<= 1;
    }

    let mut extra_data = vec![];
    loop {
        if (trig_flags & flag_accessor) != 0 {
            if data_offset >= raw_data.len() {
                warn!(
                    "Trigger has more flags than data! ({} data)",
                    raw_data.len()
                );
                extra_data = vec![];
                break;
            }

            extra_data.push(raw_data[data_offset]);
            data_offset += 1;
        } else {
            extra_data.push(u32::MAX);
        }

        if flag_accessor == (1 << 31) {
            break;
        }

        flag_accessor <<= 1;
    }

    (data, links, extra_data)
}

fn default_icon_scale() -> f32 { 0.25 }

#[derive(Default, Clone, Debug, Deserialize)]
pub struct TriggerInformation {
    #[serde(default = "default_icon_scale")]
    pub icon_scale: f32,
    #[serde(default)]
    pub extra_values: BTreeMap<u32, TriggerValue>,
    pub triggers: BTreeMap<u32, TriggerDefinition>,
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
    pub dtype: TrigDataType,
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrigDataType {
    Unknown,
    U32,
    F32,
    Hashcode,
}

impl TrigDataType {
    pub fn to_string(&self, hashcodes: &IntMap<u32, String>, v: u32) -> String {
        match self {
            TrigDataType::Unknown => {
                if (v & 0xffff0000) != 0 {
                    if let Some(hc) = hashcodes.get(&v) {
                        return format!("{hc} (0x{:x})", v);
                    }
                }
                format!("{} (0x{:x})", human_num(v), v)
            }
            TrigDataType::U32 => {
                if v > 9999 {
                    format!("0x{v:x}")
                } else {
                    format!("{v}")
                }
            }
            TrigDataType::F32 => unsafe { format!("{:.5}", transmute::<u32, f32>(v)) },
            TrigDataType::Hashcode => {
                let hashcode = hashcodes.get(&v);

                if let Some(hc) = hashcode {
                    hc.clone()
                } else {
                    // TODO(cohae): Check if the amount of type/index bits are correct
                    if let Some(hc_base) = hashcodes.get(&(v & 0x7fff0000)) {
                        let is_local = (v & 0x80000000) != 0;
                        let hc_base_stripped = hc_base
                            .strip_suffix("_HASHCODE_BASE")
                            .unwrap_or("HT_Invalid");

                        if is_local {
                            format!("HT_Local_{}_{v:08x}", &hc_base_stripped[3..])
                        } else {
                            format!("{hc_base_stripped}_Unknown_{v:08x}")
                        }
                    } else {
                        format!("HT_Invalid_{v:08x}")
                    }
                }
            }
        }
    }
}

impl Default for TrigDataType {
    fn default() -> Self {
        TrigDataType::Unknown
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
