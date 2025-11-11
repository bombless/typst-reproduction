use super::hash_u64;
use super::shapes::Shapes as _;
use super::text::Text as _;
use super::{MyApp, collect_font_from_frame};
use eframe::egui;
use egui::DroppedFile;
use egui::containers::Frame;
use egui::{Color32, FontFamily, Ui};
use typst::layout::Abs;
use typst::visualize::Color;
use typst_library::layout::FrameItem::{Group, Image, Shape, Text};
use typst_library::layout::{Frame as TypstFrame, Point};
use typst_library::text::TextItem;
use typst_library::visualize::{Geometry::Line, Paint::Solid, Shape as TypstShape};

use std::io::Read;

fn render_text(ui: &mut Ui, text: &TextItem, point: Point, display: bool) {
    // if display {
    //     if !text.glyphs.iter().any(|x| x.c.is_whitespace()) {
    //         println!("render_text {:?}", point);
    //         tracing::debug!("render_text {:?}", point);
    //         for x in &text.glyphs {
    //             print!("{:?},", x.c);
    //             tracing::debug!("{:?},", x.c);
    //         }
    //         println!();
    //     }
    // }
    //

    if display {
        super::print_font_info(text.font.ttf());
    }

    let font_hash = hash_u64(text.font.data().as_slice());
    let font_name = format!("font-{}", font_hash);
    let family = FontFamily::Name(font_name.clone().into());

    let color = match text.fill {
        Solid(color) => color,
        _ => Color::BLACK,
    };
    let rgb_color = color.to_rgb();

    let content = text.text.as_str();

    if display {
        println!(
            "draw text at ({}, {}) font size {} font_name {} content {} color {:?}",
            point.x.to_pt(),
            point.y.to_pt(),
            text.size.to_pt(),
            &font_name,
            &content,
            &rgb_color
        );
    }

    ui.draw_text(
        &content,
        point.x.to_pt() as f32,
        point.y.to_pt() as f32,
        text.size.to_pt() as f32,
        family,
        Color32::from_rgb(
            (rgb_color.red * 256.0) as _,
            (rgb_color.green * 256.0) as _,
            (rgb_color.blue * 256.0) as _,
        ),
    );
}

fn render_frame(
    ui: &mut Ui,
    frame: &TypstFrame,
    offset: Point,
    display: bool,
    line_count: &mut u32,
) {
    // if display {
    //     println!("render_frame");
    // }
    for (point, item) in frame.items() {
        if display {
            println!("one frame item");
        }
        let origin = *point + offset;
        // if display {
        //     println!("{:?} {:?}", origin, item);
        //     tracing::debug!("#{:?} {:?}", point, item);
        // }
        match item {
            Text(text) => render_text(ui, text, origin, display),
            Group(group) => render_frame(ui, &group.frame, origin, display, line_count),
            Shape(
                TypstShape {
                    geometry: Line(line_to),
                    stroke: Some(stroke),
                    ..
                },
                _,
            ) => {
                *line_count += 1;
                if *line_count > 23 {
                    return;
                }
                let color = match stroke.paint {
                    Solid(color) => color,
                    _ => Color::BLACK,
                };
                // println!("origin {:?}", origin);
                let dst = *line_to + origin;
                // println!("origin {:?}", origin);
                ui.draw_line(
                    origin.x.to_pt(),
                    origin.y.to_pt(),
                    dst.x.to_pt(),
                    dst.y.to_pt(),
                    stroke.thickness.to_pt(),
                    color,
                );
                if display {
                    tracing::debug!("draw_line {:?} {:?}", (origin, dst), color);
                    eprintln!("draw_line {:?} {:?}", (origin, dst), color);
                }
            }
            Shape(s, span) => {
                if display {
                    tracing::debug!("{:?} {:?}", s, span)
                };
            }
            Image(_, size, span) => {
                if display {
                    tracing::debug!("image {:?} {:?}", size, span);
                }
            }
            x => {
                if display {
                    tracing::debug!("wat {:?}", x);
                }
            }
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        handle_files(ctx);

        if let Some(bytes) = self.source.take() {
            tracing::debug!("self.renderer.render_from_slice(&bytes);");
            let page = self.renderer.render_from_string(bytes);
            tracing::debug!("render_from_slice done");
            collect_font_from_frame(&mut self.font_definitions, &page);
            ctx.set_fonts(self.font_definitions.clone());
            self.page = Some(page);
            println!("page update");
            ctx.request_repaint();
            return; // wait until next frame
        }

        ctx.input(|i| {
            if let Some(file) = i.raw.dropped_files.first() {
                if let DroppedFile {
                    bytes: Some(bytes), ..
                } = file
                {
                    let source = String::from_utf8(bytes.iter().copied().collect()).unwrap();
                    self.source = Some(source);
                    tracing::debug!("{} bytes", bytes.len());
                } else if let DroppedFile {
                    path: Some(path), ..
                } = file
                {
                    let mut file = std::fs::File::open(path).unwrap();
                    let mut source = String::new();
                    println!("{} bytes", source.len());
                    file.read_to_string(&mut source).unwrap();
                    self.source = Some(source);
                }
            }
        });

        let options = Frame {
            fill: Color32::WHITE,
            ..Frame::default()
        };

        egui::CentralPanel::default()
            .frame(options)
            .show(ctx, |ui| {
                ui.text_edit_multiline(&mut self.input);

                if ui.button("编译").clicked() {
                    self.source = Some(self.input.clone());
                    self.display = true;
                    ctx.request_repaint();
                    return;
                }

                if let Some(page) = &self.page {
                    let mut line_count = 0;
                    render_frame(
                        ui,
                        page,
                        Point::new(Abs::pt(0.0), Abs::pt(90.0)),
                        self.display,
                        &mut line_count,
                    );
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

        let screen_rect = ctx.content_rect();
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
