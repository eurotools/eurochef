use eurochef_edb::Hashcode;
use eurochef_shared::script::{UXGeoScriptCommand, UXGeoScriptCommandData};
use glam::{Quat, Vec3};

use crate::{map_frame::QueuedEntityRender, render::tweeny::ease_in_out_sine};

use super::RenderStore;

pub fn render_script<F>(
    current_file: Hashcode,
    position: Vec3,
    rotation: Quat,
    scale: Vec3,
    file: Hashcode,
    script_hashcode: Hashcode,
    current_time: f32,
    render_store: &RenderStore,
    mut render: F,
) where
    F: FnMut(QueuedEntityRender),
{
    let script = render_store.get_script(file, script_hashcode);
    if script.is_none() {
        return;
    }
    let script = script.unwrap();

    let current_frame = (current_time * script.framerate).floor() as isize;
    let current_frame_commands: Vec<&UXGeoScriptCommand> = script
        .commands
        .iter()
        .filter(|c| c.range().contains(&current_frame))
        .collect();

    let mut transforms = vec![];
    for cf in &current_frame_commands {
        let (pos, rot, scale) = if let Some(controller) =
            script.controllers.get(cf.controller_index as usize)
        {
            macro_rules! get_interp_pos {
                ($v:expr, $default:expr) => {{
                    let mut previous_frame = -1;
                    let mut next_frame = -1;
                    let current_frame = current_time * script.framerate;

                    for (i, (start, _)) in $v.iter().enumerate() {
                        if *start > current_frame {
                            break;
                        }

                        previous_frame = i as isize;
                    }

                    if previous_frame != -1 {
                        next_frame = previous_frame + 1;
                    }

                    if next_frame == -1 || next_frame > $v.len() as isize {
                        next_frame = 0;
                    }

                    let (start, start_value) =
                        if let Some((k, fvalue)) = $v.get(previous_frame as usize) {
                            (*k, *fvalue)
                        } else {
                            (cf.start as f32, $default)
                        };

                    let (end, end_value) = if let Some((k, fvalue)) = $v.get(next_frame as usize) {
                        (*k, *fvalue)
                    } else {
                        (start, start_value)
                    };

                    (start, start_value, end, end_value)
                }};
            }

            let rot = {
                let (start, start_rot, end, end_rot) =
                    get_interp_pos!(controller.channels.quat_0, Quat::IDENTITY.to_array());

                let length = end - start;
                let offset = ((current_time * script.framerate) - start) / length;
                if start == end {
                    Quat::from_array(start_rot)
                } else {
                    Quat::from_array(start_rot).lerp(Quat::from_array(end_rot), offset)
                }
            };

            let pos = {
                let (start, start_pos, end, end_pos) =
                    get_interp_pos!(controller.channels.vector_0, Vec3::ZERO.to_array());

                let length = end - start;
                let offset = ((current_time * script.framerate) - start) / length;
                if start == end {
                    start_pos.into()
                } else {
                    Vec3::from(start_pos).lerp(Vec3::from(end_pos), ease_in_out_sine(offset))
                }
            };

            let scale = {
                let (start, start_scale, end, end_scale) =
                    get_interp_pos!(controller.channels.vector_1, Vec3::ONE.to_array());

                let length = end - start;
                let offset = ((current_time * script.framerate) - start) / length;
                if start == end {
                    start_scale.into()
                } else {
                    Vec3::from(start_scale).lerp(Vec3::from(end_scale), offset)
                }
            };

            (pos, rot, scale)
        } else {
            (Vec3::ZERO, Quat::IDENTITY, Vec3::ONE)
        };

        transforms.push((pos, rot, scale));
    }

    for (c, transform) in current_frame_commands.iter().zip(&transforms) {
        match c.data {
            UXGeoScriptCommandData::Entity { hashcode, file } => render(QueuedEntityRender {
                entity: (if file == u32::MAX { current_file } else { file }, hashcode),
                entity_alt: None,
                position: position + transform.0,
                rotation: rotation * transform.1,
                scale: scale * transform.2,
            }),
            _ => {}
        }
    }
}
