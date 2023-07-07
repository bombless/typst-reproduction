use super::{MyApp};
use typst::geom::Paint::Solid;
use typst::doc::FrameItem::{Text, Group, Shape, Image, Meta};
use typst::geom::Geometry::Line;
use typst::geom;
use typst::doc::{Frame as TypstFrame, TextItem};
use typst::geom::Point;


fn render_text(ui: &mut MyApp, text: &TextItem, point: Point, display: bool) {

    let mut font_family = None;
    let fonts = text.font.ttf().names();
    for i in 0 .. fonts.len() {
        let font = fonts.get(i).unwrap().to_string();
        if font.is_none() { continue }
        font_family = font;
        if display {
            println!("{:?}", fonts.get(i).unwrap())
        }
    }
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

    let Solid(color) = text.fill;
    let rgba_color = color.to_rgba();

    ui.draw_text(
        &text.glyphs.iter().map(|x| x.c).collect::<String>(),
        point.x.to_pt() as f32,
        point.y.to_pt() as f32,
        text.size.to_pt() as f32,
        slint::Color::from_argb_u8(rgba_color.a, rgba_color.r, rgba_color.g, rgba_color.b),
        font_family.unwrap()
    );
}

fn render_frame(ui: &mut MyApp, frame: &TypstFrame, offset: Point, display: bool, line_count: &mut u32) {
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
                let color = color.to_rgba();
                let color = slint::Color::from_argb_u8(color.a, color.r, color.g, color.b);
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

impl MyApp {
    fn draw_text(&mut self, input: &str, x: f32, y: f32, size: f32, color: slint::Color, font_family: String) {
        use std::rc::Rc;
        self.text_items.push(super::TextItem {
            text: input.into(),
            x: x as _,
            y: y as _,
            color,
            size,
            font_family: font_family.into()
        });
    }
    fn draw_line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, thickness: f64, color: slint::Color) {
        self.line_items.push(super::LineItem {
            thickness: thickness as _,
            x1: x1 as _,
            y1: y1 as _,
            x2: x2 as _,
            y2: y2 as _,
            color,
            // font_families: slint::ModelRc::new(font_families.into_iter().map(slint::SharedString::from).collect::<Vec<_>>())
        })
    }
    pub fn update(&mut self) {
        let mut line_count = 0;
        // handle_files(ctx);
        if let Some(page) = self.page.clone() {
            render_frame(self, &page, Point::default(), true, &mut line_count);
        }
        
    }
}

// fn handle_files(ctx: &egui::Context) {
//     use egui::*;
//     use std::fmt::Write as _;

//     if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
//         let text = ctx.input(|i| {
//             let mut text = "Dropping files:\n".to_owned();
//             for file in &i.raw.hovered_files {
//                 if let Some(path) = &file.path {
//                     write!(text, "\n{}", path.display()).ok();
//                 } else if !file.mime.is_empty() {
//                     write!(text, "\n{}", file.mime).ok();
//                 } else {
//                     text += "\n???";
//                 }
//                 text += &format!("\n{:?}", file)
//             }
//             text
//         });

//         tracing::debug!("drop!");

//         let painter =
//             ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

//         let screen_rect = ctx.screen_rect();
//         painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
//         painter.text(
//             screen_rect.center(),
//             Align2::CENTER_CENTER,
//             text,
//             TextStyle::Heading.resolve(&ctx.style()),
//             Color32::GOLD,
//         );
//     }
// }