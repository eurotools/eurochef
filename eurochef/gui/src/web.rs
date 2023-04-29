// #[cfg(target_arch = "wasm32")]
// #[wasm_bindgen(module = "/assets/util.js")]
// extern "C" {
//     pub fn import_data();
// }

// use eframe::wasm_bindgen;
// use stdweb::{
//     js,
//     unstable::TryIntoWeb,
//     web::{document, event::ChangeEvent, FileList, IElement, IEventTarget},
// };

// pub fn import_data() {
//     info!("Hey");
//     let input = document().create_element("input").unwrap();
//     input.set_attribute("accept", ".edb,.sfx").unwrap();
//     input.set_attribute("type", "file").unwrap();
//     input.set_attribute("display", "none").unwrap();
//     let input_c = input.clone();
//     input_c.add_event_listener(move |_event: ChangeEvent| {
//         let files: FileList = js!( return @{input.as_ref()}.files; )
//             .try_into_web()
//             .unwrap();
//     });

//     js! { @{ input_c }.click() }
// }
