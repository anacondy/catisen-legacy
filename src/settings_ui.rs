use crate::config::{CatisenConfig, ConfigViewMode};
use egui::{Button, Color32, ComboBox, CtxRef, RichText, Ui, Window};

pub struct SettingsWindow {
    pub open: bool,
    pub config: CatisenConfig,
    pub original_config: CatisenConfig,
    status_message: String,
}

impl SettingsWindow {
    pub fn new(config: CatisenConfig) -> Self {
        Self {
            open: false,
            original_config: config.clone(),
            config,
            status_message: String::new(),
        }
    }

    pub fn show(&mut self, ctx: &CtxRef) -> bool {
        let mut changed = false;
        if self.open {
            Window::new("Settings")
                .open(&mut self.open)
                .resizable(true)
                .default_width(400.0)
                .show(ctx, |ui| {
                    self.ui(ui);
                    if self.config != self.original_config {
                        changed = true;
                    }
                });
        }
        changed
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        ui.heading("Privacy");

        ui.checkbox(&mut self.config.use_tor_by_default, "Use Tor by default");
        ui.horizontal(|ui| {
            ui.label("Tor Proxy URL:");
            ui.text_edit_singleline(&mut self.config.tor_proxy_url);
        });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        ui.heading("Appearance");
        ui.horizontal(|ui| {
            ui.label("Theme:");
            ui.text_edit_singleline(&mut self.config.theme);
        });
        ui.horizontal(|ui| {
            ui.label("Default View Mode:");
            ComboBox::from_label("")
                .selected_text(format!("{:?}", self.config.default_view_mode))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.config.default_view_mode,
                        ConfigViewMode::Text,
                        "Text",
                    );
                    ui.selectable_value(
                        &mut self.config.default_view_mode,
                        ConfigViewMode::Source,
                        "Source",
                    );
                    ui.selectable_value(
                        &mut self.config.default_view_mode,
                        ConfigViewMode::Visual,
                        "Visual",
                    );
                });
        });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        ui.heading("Behavior");
        ui.horizontal(|ui| {
            ui.label("Home Page:");
            ui.text_edit_singleline(&mut self.config.home_page);
        });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        ui.heading("Performance");
        ui.horizontal(|ui| {
            ui.label("Target FPS:");
            ui.add(egui::Slider::new(&mut self.config.target_fps, 15..=120));
        });
        ui.checkbox(&mut self.config.fps_telemetry, "Show FPS Telemetry");

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            if ui.button("Save").clicked() {
                match self.config.save() {
                    Ok(_) => {
                        self.status_message = "Settings saved!".to_string();
                        self.original_config = self.config.clone();
                    }
                    Err(e) => {
                        self.status_message = format!("Error saving: {}", e);
                    }
                }
            }

            if ui.button("Reset").clicked() {
                self.config = CatisenConfig::default();
            }

            if !self.status_message.is_empty() {
                let color = if self.status_message.starts_with("Error") {
                    Color32::RED
                } else {
                    Color32::GREEN
                };
                ui.label(RichText::new(&self.status_message).color(color));
            }
        });
    }
}

