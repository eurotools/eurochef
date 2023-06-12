use serde::Serialize;
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
    pub data: Vec<u32>,
    pub links: Vec<i32>,
    pub extra_data: Vec<u32>,
}

// TODO(cohae): Move this to eurochef-edb so we can parse trigger collisions
pub fn parse_trigger_data(
    _version: u32,
    trig_flags: u32,
    raw_data: &[u32],
) -> (Vec<u32>, Vec<i32>, Vec<u32>) {
    let mut data = vec![];
    let mut links = vec![];

    let mut flag_accessor = 1;
    let mut data_offset = 0;

    // TODO(cohae): Some older games use only 8 values instead of 16
    for _ in 0..16 {
        if (trig_flags & flag_accessor) != 0 {
            data.push(raw_data[data_offset]);
            data_offset += 1;
        } else {
            data.push(0);
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
