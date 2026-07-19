
#[cfg(feature = "egui_ui")]
pub fn test(ctx: &eframe::egui::Context, id: eframe::egui::Id) {
    if let Some(state) = eframe::egui::TextEdit::load_state(ctx, id) {
        if let Some(cr) = state.cursor_range() {
        }
    }
}
