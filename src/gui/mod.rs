use eframe::egui::{FontData, FontDefinitions};
use eframe::epaint::FontFamily;
use std::path::PathBuf;
use typst::doc::Frame;
use typst::doc::FrameItem::{Group, Text};

use rusttype::{point, Font, Scale};
use ttf_parser::Face;

use std::sync::Arc;

mod shapes;
mod text;
mod update;

enum View {
    Tree,
    Text,
}

enum TreeNode {
    Leaf(String),
    Node(Vec<TreeNode>),
}

struct MyApp {
    page: Option<Frame>,
    renderer: super::Renderer,
    display: bool,
    bytes: Option<Vec<u8>>,
    line_count: u32,
    view: View,
    tree: Option<TreeNode>,
    input: String,
}

impl MyApp {
    fn new(renderer: super::Renderer) -> Self {
        MyApp {
            page: None,
            renderer,
            display: true,
            bytes: None,
            line_count: 0,
            view: View::Text,
            tree: None,
            input: "#v(100pt)\n#line(length:100%)\n= 你好，世界".into(),
        }
    }
}

pub(crate) fn run(file: Option<PathBuf>, mut renderer: super::Renderer) {
    let mut options = eframe::NativeOptions::default();

    let page = file.map(|x| renderer.render_from_path(&x));
    let mut app = MyApp::new(renderer);

    let mut font_definitions = FontDefinitions::default();
    if page.is_none() && !app.input.is_empty() {
        let page = app.renderer.render_from_vec(app.input.as_bytes().into());
        collect_font_from_frame(&mut font_definitions, &page);
        app.page = Some(page);
    }

    if let Some(page) = &page {
        let x = page.width().to_pt() as f32;
        let y = page.height().to_pt() as f32;

        collect_font_from_frame(&mut font_definitions, page);
    }

    eframe::run_native(
        "litter typer",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_fonts(font_definitions);
            Ok(Box::new(app))
        }),
    )
    .unwrap()
}

pub(crate) fn collect_font_from_frame(defs: &mut FontDefinitions, frame: &Frame) {
    for (_, item) in frame.items() {
        match item {
            Text(text) => {
                if !char_in_font(text.font.data().as_slice(), '你') {
                    continue;
                }
                let font_ptr = unsafe { std::mem::transmute::<_, usize>(text.font.data()) };
                let font_name = format!("font-{}", font_ptr);
                println!("font {}", font_name);
                let data = make('你', text.font.data().as_slice());
                for line in data {
                    for item in line {
                        print!("{item}");
                    }
                    println!();
                }
                if !defs.font_data.contains_key(&font_name) {
                    defs.font_data.insert(
                        "chinese".to_owned(),
                        Arc::new(FontData::from_owned(text.font.data().to_vec())),
                    );
                    defs.families
                        .entry(FontFamily::Proportional)
                        .or_default()
                        .insert(0, "chinese".to_owned());
                }
            }
            Group(group) => collect_font_from_frame(defs, &group.frame),
            _ => (),
        }
    }
}

fn make(c: char, font_data: &[u8]) -> [[char; 64]; 32] {
    // This only succeeds if collection consists of one font
    let font = Font::try_from_bytes(font_data).expect("Error constructing Font");

    // The font size to use
    let scale = Scale { x: 64.0, y: 32.0 };

    let v_metrics = font.v_metrics(scale);
    // println!("v_metrics {v_metrics:?}");

    let mut data = [[' '; 64]; 32];
    let cursor = point(0.0, v_metrics.ascent);
    let glyph = font.glyph(c);
    let scaled = glyph.scaled(scale);
    let glyph = scaled.positioned(cursor);
    if let Some(bounding_box) = glyph.pixel_bounding_box() {
        // Draw the glyph into the image per-pixel by using the draw closure
        glyph.draw(|x, y, v| {
            let x = x as i32 + bounding_box.min.x + 1;
            let y = y as i32 + bounding_box.min.y;
            if x >= 0 && y >= 0 && x < 64 && y < 32 {
                let x = x as usize;
                let y = y as usize;
                let print = if v >= 0.5 {
                    '@'
                } else if v >= 0.25 {
                    '$'
                } else if v >= 0.125 {
                    '+'
                } else {
                    ' '
                };
                data[y][x] = print;
            } else {
                println!("Out of bounds: ({}, {}) limit (64, 32)", x, y,);
            }
        });
    }
    data
}
fn char_in_font(data: &[u8], ch: char) -> bool {
    let face = Face::parse(data, 0).unwrap();
    face.glyph_index(ch).is_some()
}
