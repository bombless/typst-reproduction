use eframe::egui::{Color32, CornerRadius, Pos2, Rect, Stroke, StrokeKind, Ui};

use typst_library::visualize::Color;

pub trait Shapes {
    fn draw_rectangle_lines(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        thickness: f64,
        color: Color32,
    );
    fn draw_rectangle(&mut self, x: f64, y: f64, w: f64, h: f64, color: Color32);
    fn draw_line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, thickness: f64, color: Color);
}

impl Shapes for Ui {
    fn draw_rectangle_lines(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        thickness: f64,
        color: Color32,
    ) {
        let min_pos = Pos2 {
            x: x as f32,
            y: y as f32,
        };
        let max_pos = Pos2 {
            x: x as f32 + w as f32,
            y: y as f32 + h as f32,
        };

        let rect = Rect {
            min: min_pos,
            max: max_pos,
        };

        let stroke = Stroke {
            width: thickness as _,
            color,
        };

        self.painter()
            .rect_stroke(rect, CornerRadius::default(), stroke, StrokeKind::Middle);
    }
    fn draw_rectangle(&mut self, x: f64, y: f64, w: f64, h: f64, color: Color32) {
        let min_pos = Pos2 {
            x: x as f32,
            y: y as f32,
        };
        let max_pos = Pos2 {
            x: x as f32 + w as f32,
            y: y as f32 + h as f32,
        };

        let rect = Rect {
            min: min_pos,
            max: max_pos,
        };

        let color = color;

        self.painter()
            .rect_filled(rect, CornerRadius::default(), color);
    }
    fn draw_line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, thickness: f64, color: Color) {
        let coord = [Pos2::new(x1 as _, y1 as _), Pos2::new(x2 as _, y2 as _)];
        let rgb = color.to_rgb();
        let stroke = Stroke {
            width: thickness as _,
            color: Color32::from_rgb(
                (rgb.red * 256.0) as u8,
                (rgb.green * 256.0) as u8,
                (rgb.blue * 256.0) as u8,
            ),
        };
        self.painter().line_segment(coord, stroke);
    }
}
