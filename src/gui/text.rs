use eframe::egui::{Color32, Pos2, Ui, Align2, FontFamily, FontId};

pub trait Text {
    fn draw_text(&mut self, text: &str, x: f32, y: f32, font_size: f32, font_family: FontFamily, color: Color32);
}

impl Text for Ui {
    fn draw_text(&mut self, text: &str, x: f32, y: f32, font_size: f32, font_family: FontFamily, color: Color32) {
        let pos = Pos2 { x, y };

        let anchor = Align2::LEFT_TOP;

        let text = text;

        let font_id = FontId::new(font_size, font_family);

        let text_color = color;

        self.painter().text(pos, anchor, text, font_id, text_color);
    }
}
