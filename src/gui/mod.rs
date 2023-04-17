use typst::doc::Frame;
use eframe::{egui, epaint::FontFamily};
use typst::doc::FrameItem::{Text, Group};
use eframe::egui::{FontDefinitions, FontData};
use std::path::PathBuf;

mod shapes;
mod text;
mod update;

struct MyApp {
    page: Option<Frame>,
    renderer: super::Renderer,
    display: bool,
    font_definitions: FontDefinitions,
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn run(file: Option<PathBuf>, mut renderer: super::Renderer) {
    let mut options = eframe::NativeOptions::default();

    let page = file.map(|x| renderer.render_from_path(&x));
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
        Box::new(move |cc| {
            
            cc.egui_ctx.set_fonts(defs.clone());
            Box::new(MyApp { page, renderer, font_definitions: defs, display: true })
        }),
    ).unwrap()
}


// when compiling to web using trunk.
#[cfg(target_arch = "wasm32")]
pub(crate) fn run(_file: Option<PathBuf>, mut renderer: super::Renderer) {
    let mut defs = FontDefinitions::default();
    
    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::start_web(
            "the_canvas_id", // hardcode it
            web_options,
            Box::new(|cc| Box::new(MyApp { page: None, renderer, font_definitions: defs, display: true })),
        )
        .await
        .expect("failed to start eframe");
    });
}

pub(crate) fn collect_font_from_frame(defs: &mut FontDefinitions, frame: &Frame) {
    for (_, item) in frame.items() {
        match item {
            Text(text) => {
                let font_ptr = unsafe { std::mem::transmute::<_, usize>(text.font.data()) };
                let font_name = format!("font-{}", font_ptr);
                println!("font {}", font_name);
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