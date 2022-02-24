pub trait ColorUtils {
    /// Multiplies the color rgb values by `factor`, keeping alpha untouched.
    fn lighten(&self, factor: f32) -> Self;
}

impl ColorUtils for egui::Color32 {
    fn lighten(&self, factor: f32) -> Self {
        egui::Color32::from_rgba_premultiplied(
            (self.r() as f32 * factor) as u8,
            (self.g() as f32 * factor) as u8,
            (self.b() as f32 * factor) as u8,
            self.a(),
        )
    }
}
