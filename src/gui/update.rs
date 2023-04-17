use super::text::Text as _;
use super::shapes::Shapes as _;
use super::{MyApp, collect_font_from_frame};
use eframe::egui;
use egui::{Color32, FontFamily, Ui};
use typst::geom::Paint::Solid;
use egui::containers::Frame;
use egui::DroppedFile;
use typst::doc::FrameItem::{Text, Group, Shape, Image, Meta};
use typst::geom::Geometry::Line;
use typst::geom;
use typst::doc::{Frame as TypstFrame, TextItem};
use typst::geom::Point;


fn render_text(ui: &mut Ui, text: &TextItem, point: Point, display: bool) {
    if display {
        if !text.glyphs.iter().any(|x| x.c.is_whitespace()) {
            println!("render_text {:?}", point);
            tracing::debug!("render_text {:?}", point);
            for x in &text.glyphs {
                print!("{:?},", x.c);
                tracing::debug!("{:?},", x.c);
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

fn render_frame(ui: &mut Ui, frame: &TypstFrame, offset: Point, display: bool, line_count: &mut u32) {
    for (point, item) in frame.items() {
        let origin = *point + offset;
        if display {
            println!("{:?} {:?}", origin, item);
            tracing::debug!("{:?} {:?}", point, item);
        }
        match item {
            Text(text) => render_text(ui, text, origin, display),
            Group(group) => render_frame(ui, &group.frame, origin, display, line_count),
            Shape(geom::Shape { geometry: Line(line_to), stroke: Some(stroke), .. }, _) => {
                *line_count += 1;
                if *line_count > 3 { return }
                let Solid(color) = stroke.paint;
                println!("origin {:?}", origin);
                let dst = *line_to + origin;
                println!("origin {:?}", origin);
                ui.draw_line(origin.x.to_pt(), origin.y.to_pt(), dst.x.to_pt(), dst.y.to_pt(), stroke.thickness.to_pt(), color);
                if display {
                    tracing::debug!("draw_line {:?} {:?}", (origin, dst), color);
                    eprintln!("draw_line {:?} {:?}", (origin, dst), color);
                }
            },
            Shape(s, span) => {
                if display { tracing::debug!("{:?} {:?}", s, span) };
            },
            Image(_, size, span) => {
                if display { tracing::debug!("image {:?} {:?}", size, span); }
            },
            Meta(meta, _) => {
                if display { tracing::debug!("meta {:?}", meta); }
            },
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        handle_files(ctx);

        if let Some(bytes) = self.bytes.take() {
            tracing::debug!("self.renderer.render_from_slice(&bytes);");
            let page = self.renderer.render_from_vec(bytes);
            tracing::debug!("render_from_slice done");
            collect_font_from_frame(&mut self.font_definitions, &page);
            ctx.set_fonts(self.font_definitions.clone());
            self.page = Some(page);
            ctx.request_repaint();
            return; // wait until next frame
        }

        ctx.input(|i| {
            if let Some(file) = i.raw.dropped_files.first() {
                if let DroppedFile { bytes: Some(bytes), .. } = file {
                    self.bytes = Some(bytes.iter().copied().collect());
                    tracing::debug!("{} bytes", bytes.len());
                } else if let DroppedFile { path: Some(path), .. } = file {
                    let file = std::fs::read(path).unwrap();
                    println!("{} bytes", file.len());
                    self.bytes = Some(file);
                }
            }
        });

        let options = Frame {
            fill: Color32::WHITE,
            ..Frame::default()
        };

        egui::CentralPanel::default().frame(options).show(ctx, |ui| {

            ui.text_edit_multiline(&mut self.input);

            if ui.button("编译").clicked() {
                self.bytes = Some(self.input.as_bytes().into());
                self.display = true;
                ctx.request_repaint();
                return;
            }

            if let Some(page) = &self.page {
                render_frame(ui, page, Point::default(), self.display, &mut self.line_count);
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