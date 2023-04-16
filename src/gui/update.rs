use super::text::Text as _;
use super::MyApp;
use eframe::egui;
use egui::{Color32, FontFamily, Ui};
use typst::geom::Paint::Solid;
use egui::containers::Frame;
use typst::doc::FrameItem::{Text, Group};
use typst::doc::{Frame as TypstFrame, TextItem};
use typst::geom::Point;


fn render_text(ui: &mut Ui, text: &TextItem, point: Point, display: bool) {
    if display {
        if !text.glyphs.iter().any(|x| x.c.is_whitespace()) {
            println!("render_text {:?}", point);
            for x in &text.glyphs {
                print!("{:?},", x.c);
            }
            println!();
        }
    }
    

    let font_ptr = unsafe { std::mem::transmute::<_, usize>(text.font.data()) };
    let font_name = format!("font-{}", font_ptr);
    let family = FontFamily::Name(font_name.into());

    let Solid(color) = text.fill;
    let rgba_color = color.to_rgba();

    ui.draw_text(
        &text.glyphs.iter().map(|x| x.c).collect::<String>(),
        point.x.to_pt() as f32,
        point.y.to_pt() as f32,
        text.size.to_pt() as f32,
        family,
        Color32::from_rgba_unmultiplied(rgba_color.r, rgba_color.g, rgba_color.b, rgba_color.a),
    );
}

fn render_frame(ui: &mut Ui, frame: &TypstFrame, offset: Point, display: bool) {
    for (mut point, item) in frame.items() {
        if display { println!("{:?}", point); }
        point += offset;
        match item {
            Text(text) => render_text(ui, text, point, display),
            Group(group) => render_frame(ui, &group.frame, point, display),
            _ => (),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {

        let options = Frame {
            fill: Color32::WHITE,
            ..Frame::default()
        };

        egui::CentralPanel::default().frame(options).show(ctx, |ui| {
            if let Some(page) = &self.page {
                render_frame(ui, page, Point::default(), self.display);
                self.display = false;
            }
        });
        
    }
}
