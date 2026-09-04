use eframe::egui;

pub(crate) fn start() {
  let _ = eframe::run_native(
    "Whirling Press",
    eframe::NativeOptions::default(),
    Box::new(|cc| Ok(Box::new(WhirlApp::new(cc)))),
  );
}

struct WhirlApp {
  root_path: Option<std::path::PathBuf>,
}

impl WhirlApp {
  fn new(cc: &eframe::CreationContext<'_>) -> Self {
    let ctx = &cc.egui_ctx;

    use egui::{FontDefinitions, FontData, FontFamily, FontId};
    use egui::style::TextStyle;

    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert("f1".to_string(),
      std::sync::Arc::new(FontData::from_static(include_bytes!("../res/InstrumentSans_Regular.ttf"))));
    fonts.font_data.insert("f9".to_string(),
      std::sync::Arc::new(FontData::from_static(include_bytes!("../res/DroidSansFallbackFull.ttf"))));
    fonts.families.get_mut(&FontFamily::Proportional).unwrap().push("f1".to_owned());
    fonts.families.get_mut(&FontFamily::Proportional).unwrap().push("f9".to_owned());
    ctx.set_fonts(fonts);

    let text_styles: std::collections::BTreeMap<_, _> = [
      (TextStyle::Heading, FontId::new(30.0, FontFamily::Proportional)),
      (TextStyle::Body, FontId::new(18.0, FontFamily::Proportional)),
      (TextStyle::Monospace, FontId::new(18.0, FontFamily::Proportional)),
      (TextStyle::Button, FontId::new(18.0, FontFamily::Proportional)),
      (TextStyle::Small, FontId::new(14.0, FontFamily::Proportional)),
    ].into();
    ctx.all_styles_mut(move |style| style.text_styles = text_styles.clone());

    Self {
      root_path: None,
    }
  }

  fn ui_landing(&mut self, ui: &mut egui::Ui) {
    let dropped_files = ui.ctx().input(|i| i.raw.dropped_files.clone());
    if !dropped_files.is_empty() {
      let file = &dropped_files[0];
      let (name, bytes) = (
        file.path().to_path_buf(),
        file.bytes().unwrap(),
      );
      println!("{} {}", name.display(), bytes.len());
    }

    use egui::{Layout, Align};

    egui::CentralPanel::default().show(ui, |ui| {
      ui.with_layout(Layout::top_down(Align::Center), |ui| {
        ui.add_space(ui.available_height() * 0.3);
        ui.heading("Whirling Press 飞旋印刷厂");
        ui.add_space(18.0);
        ui.separator();
        ui.add_space(18.0);
        if ui.button("Open content folder").clicked()
          && let Some(path) = rfd::FileDialog::new().pick_folder() {
          self.root_path = Some(path.to_path_buf());
          ui.ctx().send_viewport_cmd(egui::ViewportCommand::Title(
            format!("{} — Whirling Press", path.display())));
        }
        ui.add_space(6.0);
        ui.label("or drag-and-drop");
      });
    });
  }

  fn ui_edit(&mut self, ui: &mut egui::Ui) {
    let root_path = self.root_path.as_ref().unwrap();
  }
}

impl eframe::App for WhirlApp {
  fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
    if let Some(_) = self.root_path {
      self.ui_edit(ui);
    } else {
      self.ui_landing(ui);
    }
  }
}
