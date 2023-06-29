use typst::doc::Frame;
use typst::doc::FrameItem::{Text, Group};
use std::path::PathBuf;

mod shapes;
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
    // font_definitions: FontDefinitions,
    bytes: Option<Vec<u8>>,
    line_count: u32,
    view: View,
    tree: Option<TreeNode>,
    input: String,
    text_items: Vec<TextItem>,
}

impl MyApp {
    fn new(renderer: super::Renderer) -> Self {
        MyApp {
            page: None,
            renderer,
            // font_definitions: FontDefinitions::default(),
            display: true,
            bytes: None,
            line_count: 0,
            view: View::Text,
            tree: None,
            input: "#v(100pt)\n#line(length:100%)\n= 你好，世界".into(),
            text_items: vec![],
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn run(file: Option<PathBuf>, mut renderer: super::Renderer) {
    use std::rc::Rc;

    let page = file.map(|x| renderer.render_from_path(&x));
    let mut app = MyApp { ..MyApp::new(renderer)};
    if app.page.is_none() && !app.input.is_empty() {
        let page = app.renderer.render_from_vec(app.input.as_bytes().into());
        // collect_font_from_frame(&mut app.font_definitions, &page);
        app.page = Some(page);
        println!("Some(page)")
    }

    let window = MainWindow::new().unwrap();
    app.update();
    println!("{} items", app.text_items.len());
    for item in &app.text_items {
        println!("{:?}", item)
    }
    let text_model = Rc::new(slint::VecModel::<TextItem>::from(app.text_items));
    window.set_text_model(text_model.into());


    if let Some(page) = &app.page {
        let x = page.width().to_pt() as f32;
        let y = page.height().to_pt() as f32;

        window.set_float_width(x);
        window.set_float_height(y);
        window.run().unwrap();

        // options.initial_window_size = Some(egui::vec2(x, y));

        // collect_font_from_frame(&mut app.font_definitions, page);
    }

}

slint::slint! {
    export struct TextItem  {
        text: string,
        x: int,
        y: int,
    }

    export component MainWindow inherits Window {
        in property <[TextItem]> text-model: [];
        in property <float> float-width: 400;
        in property <float> float-height: 300;
        width: float-width * 1px;
        height: float-height * 1px;
        
        for item in root.text-model: Text {
            x: item.x * 1px;
            y: item.y * 1px;
            text: item.text;
        }
    }
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
            Box::new(|cc| Box::new(MyApp::new(renderer))),
        )
        .await
        .expect("failed to start eframe");
    });
}

// pub(crate) fn collect_font_from_frame(defs: &mut FontDefinitions, frame: &Frame) {
//     for (_, item) in frame.items() {
//         match item {
//             Text(text) => {
//                 let font_ptr = unsafe { std::mem::transmute::<_, usize>(text.font.data()) };
//                 let font_name = format!("font-{}", font_ptr);
//                 println!("font {}", font_name);
//                 if !defs.font_data.contains_key(&font_name) {
//                     defs.font_data.insert(font_name.clone(), FontData::from_owned(text.font.data().to_vec()));
//                     defs.families.insert(FontFamily::Name(font_name.clone().into()), vec![font_name.clone()]);
//                 }
//             },
//             Group(group) => collect_font_from_frame(defs, &group.frame),
//             _ => (),
//         }
//     }
// }