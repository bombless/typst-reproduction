use eframe::egui::{Stroke, Rect, Pos2, Rounding, Color32, Ui};
use typst::geom::Color;

pub trait Shapes {
    fn draw_rectangle_lines(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        thickness: f64,
        color: Color32
    );
    fn draw_rectangle(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        color: Color32
    );
    fn draw_line(
        &mut self,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        thickness: f64,
        color: Color
    );
}

impl Shapes for Ui {
    fn draw_rectangle_lines(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        thickness: f64,
        color: Color32
    ) {
        let min_pos = Pos2 { x: x as f32, y: y as f32 };
        let max_pos = Pos2 { x: x as f32 + w as f32, y: y as f32 + h as f32 };

        let rect = Rect { min: min_pos, max: max_pos };

        let rounding = Rounding::none();

        let stroke = Stroke {
            width: thickness as _,
            color,
        };

        self.painter().rect_stroke(rect, rounding, stroke);
    }
    fn draw_rectangle(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        color: Color32
    ) {
        let min_pos = Pos2 { x: x as f32, y: y as f32 };
        let max_pos = Pos2 { x: x as f32 + w as f32, y: y as f32 + h as f32 };

        let rect = Rect { min: min_pos, max: max_pos };

        let rounding = Rounding::none();

        let color = color;

        self.painter().rect_filled(rect, rounding, color);
    }
    fn draw_line(
        &mut self,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        thickness: f64,
        color: Color
    ) {
        let coord = [Pos2::new(x1 as _, y1 as _), Pos2::new(x2 as _, y2 as _)];
        let rgba = color.to_rgba();
        let stroke = Stroke { width: thickness as _, color: Color32::from_rgba_premultiplied(rgba.r, rgba.g, rgba.b, rgba.a) };
        self.painter().line_segment(coord, stroke);
    }
}