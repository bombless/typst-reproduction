use typst::doc::Frame;
use eframe::{egui, epaint::FontFamily};
use typst::doc::FrameItem::{Text, Group};
use eframe::egui::{FontDefinitions, FontData};
use std::path::PathBuf;

mod shapes;
mod text;
mod update;

#[derive(Default)]
struct MyApp {
    page: Option<Frame>,
    display: bool,
}

pub(crate) fn run(file: Option<PathBuf>, open: &mut dyn for<'a> FnMut(PathBuf) -> Frame) {
    let mut options = eframe::NativeOptions::default();

    let page = file.map(open);
    let mut defs = FontDefinitions::default();

    if let Some(page) = &page {
        let x = page.width().to_pt() as f32;
        let y = page.height().to_pt() as f32;

        options.initial_window_size = Some(egui::vec2(x, y));

        collect_font_from_frame(&mut defs, page);
    }

    eframe::run_native(
        "litter typer",
        options,
        Box::new(|cc| {
            
            cc.egui_ctx.set_fonts(defs);
            Box::new(MyApp { page, display: true })
        }),
    ).unwrap()
}

fn collect_font_from_frame(defs: &mut FontDefinitions, frame: &Frame) {
    for (_, item) in frame.items() {
        match item {
            Text(text) => {
                let font_ptr = unsafe { std::mem::transmute::<_, usize>(text.font.data()) };
                let font_name = format!("font-{}", font_ptr);
                if !defs.font_data.contains_key(&font_name) {
                    defs.font_data.insert(font_name.clone(), FontData::from_owned(text.font.data().to_vec()));
                    defs.families.insert(FontFamily::Name(font_name.clone().into()), vec![font_name.clone()]);
                }
            },
            Group(group) => collect_font_from_frame(defs, &group.frame),
            _ => (),
        }
    }
}