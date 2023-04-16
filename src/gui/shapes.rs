use eframe::egui::{Stroke, Rect, Pos2, Rounding, Color32, Ui};

pub trait Shapes {
    fn draw_rectangle_lines(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        thickness: f32,
        color: Color32
    );
    fn draw_rectangle(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: Color32
    );
}

impl Shapes for Ui {
    fn draw_rectangle_lines(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        thickness: f32,
        color: Color32
    ) {
        let min_pos = Pos2 { x, y };
        let max_pos = Pos2 { x: x + w, y: y + h };

        let rect = Rect { min: min_pos, max: max_pos };

        let rounding = Rounding::none();

        let stroke = Stroke {
            width: thickness,
            color,
        };

        self.painter().rect_stroke(rect, rounding, stroke);
    }
    fn draw_rectangle(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: Color32
    ) {
        let min_pos = Pos2 { x, y };
        let max_pos = Pos2 { x: x + w, y: y + h };

        let rect = Rect { min: min_pos, max: max_pos };

        let rounding = Rounding::none();

        let color = color;

        self.painter().rect_filled(rect, rounding, color);
    }
}