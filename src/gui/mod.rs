mod shapes;
mod text;
mod update;

use eframe::egui::{FontData, FontDefinitions};
use eframe::epaint::FontFamily;
use std::path::PathBuf;
use typst_library::layout::Frame;
use typst_library::layout::FrameItem::{Group, Text};

use std::sync::Arc;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use ttf_parser::{Face, name_id};

fn hash_u64(data: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}

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
    source: Option<String>,
    view: View,
    tree: Option<TreeNode>,
    input: String,
    font_definitions: FontDefinitions,
}

impl MyApp {
    fn new(renderer: super::Renderer, input: String) -> Self {
        MyApp {
            page: None,
            renderer,
            display: true,
            source: None,
            view: View::Text,
            tree: None,
            // input: "#v(100pt)\n#line(length:100%)\n= 你好，世界233".into(),
            input,
            font_definitions: FontDefinitions::empty(),
        }
    }
}

pub(crate) fn run(file: Option<PathBuf>, mut renderer: super::Renderer) {
    use std::fs::File;
    use std::io::Read;

    let options = eframe::NativeOptions::default();

    let page = file.as_ref().map(|x| renderer.render_from_path(&x));
    let input = file.map_or_else(String::new, |x| {
        let mut f = File::open(x).unwrap();
        let mut ret = String::new();
        f.read_to_string(&mut ret).unwrap();
        ret
    });
    let mut app = MyApp::new(renderer, input);

    let mut font_definitions = FontDefinitions::default();
    // if page.is_none() && !app.input.is_empty() {
    //     let page = app.renderer.render_from_vec(app.input.as_bytes().into());
    //     collect_font_from_frame(&mut app.font_definitions, &page);
    //     app.page = Some(page);
    // }

    if let Some(page) = &page {
        collect_font_from_frame(&mut font_definitions, &page);
    }
    app.page = page;

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

fn make(c: char, font_data: &[u8]) -> [[char; 64]; 32] {
    use rusttype::{Font, Scale, point};
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

pub(crate) fn collect_font_from_frame(defs: &mut FontDefinitions, frame: &Frame) {
    for (_, item) in frame.items() {
        match item {
            Text(text) => {
                let font_hash = hash_u64(text.font.data().as_slice());
                let font_name = format!("font-{}", font_hash);

                if !defs.font_data.contains_key(&font_name) {
                    println!("=== font {font_name}");
                    let data = make('2', text.font.data().as_slice());
                    let mut empty = true;
                    for line in data {
                        for item in line {
                            if item != ' ' {
                                empty = false;
                            }
                            print!("{item}");
                        }
                        println!();
                    }
                    println!("'2' is {}", if empty { "empty" } else { "not empty" });

                    let face = Face::parse(text.font.data().as_slice(), 0).unwrap();
                    print_font_info(&face);
                    if char_in_font(&face, '你') {
                        defs.font_data.insert(
                            "chinese".to_owned(),
                            Arc::new(FontData::from_owned(text.font.data().to_vec())),
                        );
                        defs.families
                            .entry(FontFamily::Proportional)
                            .or_default()
                            .insert(0, "chinese".to_owned());
                    }
                    println!("##done");
                    defs.font_data.insert(
                        font_name.to_owned(),
                        Arc::new(FontData::from_owned(text.font.data().to_vec())),
                    );
                    println!("#font family {font_name}");
                    defs.families
                        .entry(FontFamily::Name(font_name.clone().into()))
                        .or_default()
                        .insert(0, font_name.to_owned());
                }
            }
            Group(group) => collect_font_from_frame(defs, &group.frame),
            _ => (),
        }
    }
}

fn char_in_font(face: &Face, ch: char) -> bool {
    face.glyph_index(ch).is_some()
}

pub fn print_font_info(face: &Face) {
    if face.glyph_index('2').is_none() {
        println!("damn, '2' is not present");
    } else {
        println!("'2' is present");
    }

    let names = face.names();

    let family = names
        .get(name_id::TYPOGRAPHIC_FAMILY)
        .or_else(|| names.get(name_id::FAMILY))
        .map(|x| x.name)
        .unwrap_or(b"?");
    let subfamily = names
        .get(name_id::TYPOGRAPHIC_SUBFAMILY)
        .or_else(|| names.get(name_id::SUBFAMILY))
        .map(|x| x.name)
        .unwrap_or(b"?");
    let full_name = names
        .get(name_id::FULL_NAME)
        .map(|x| x.name)
        .unwrap_or(b"?");
    let postscript = names
        .get(name_id::POST_SCRIPT_NAME)
        .map(|x| x.name)
        .unwrap_or(b"?");

    println!(
        "size {:.2}MB",
        face.raw_face().data.len() as f64 / 1_000_000.0
    );

    println!("Family     : {}", String::from_utf8_lossy(family));
    println!("Subfamily  : {}", String::from_utf8_lossy(subfamily));
    println!("Full name  : {}", String::from_utf8_lossy(full_name));
    println!("PostScript : {}", String::from_utf8_lossy(postscript));
}
