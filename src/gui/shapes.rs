
use typst::geom::Color;

pub trait Shapes {
    fn draw_rectangle_lines(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        thickness: f64,
        color: Color
    );
    fn draw_rectangle(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        color: Color
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
