use super::text::Text as _;
use super::MyApp;
use eframe::egui;
use egui::{Color32, FontFamily, Ui};
use typst::geom::Paint::Solid;
use egui::containers::Frame;
use egui::DroppedFile;
use typst::doc::FrameItem::{Text, Group};
use typst::doc::{Frame as TypstFrame, TextItem};
use typst::geom::Point;
use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use rfd::FileDialog;


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
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {

                if let Some(DroppedFile { bytes: Some(bytes), .. }) = i.raw.dropped_files.first() {
                    let data_url = format!("data:;base64,{}", base64_url::encode(&bytes));
                    let path = Path::new(&data_url);
                    tracing::debug!("drop! {:?}", path);
                    let page = (self.renderer.borrow_mut())(path.to_path_buf());
                    self.page = Some(page);
                    return; // wait until next frame
                }
            }
        });
        handle_files(ctx);

        let options = Frame {
            fill: Color32::WHITE,
            ..Frame::default()
        };

        egui::CentralPanel::default().frame(options).show(ctx, |ui| {

            if ui.button("Open file…").clicked() {
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(path) = FileDialog::new().add_filter("typst source file", &["typ"]).pick_file() {
                    let page = (self.renderer.borrow_mut())(path);
                    super::collect_font_from_frame(&mut self.font_definitions, &page);
                    ctx.set_fonts(self.font_definitions.clone());
                    self.page = Some(page);
                    return; // wait until next frame
                }
            }

            if let Some(page) = &self.page {
                render_frame(ui, page, Point::default(), self.display);
                self.display = false;
            }
        });
        
    }
}

fn handle_files(ctx: &egui::Context) {
    use egui::*;
    use std::fmt::Write as _;

    if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
        let text = ctx.input(|i| {
            let mut text = "Dropping files:\n".to_owned();
            for file in &i.raw.hovered_files {
                if let Some(path) = &file.path {
                    write!(text, "\n{}", path.display()).ok();
                } else if !file.mime.is_empty() {
                    write!(text, "\n{}", file.mime).ok();
                } else {
                    text += "\n???";
                }
                text += &format!("\n{:?}", file)
            }
            text
        });

        tracing::debug!("drop!");

        let painter =
            ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

        let screen_rect = ctx.screen_rect();
        painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
        painter.text(
            screen_rect.center(),
            Align2::CENTER_CENTER,
            text,
            TextStyle::Heading.resolve(&ctx.style()),
            Color32::GOLD,
        );
    }
}