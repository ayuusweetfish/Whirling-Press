use eframe::egui;

pub(crate) fn start() {
  let native_options = eframe::NativeOptions::default();
  eframe::run_native("Whirling Press", native_options, Box::new(|cc| Ok(Box::new(WhirlApp::new(cc)))));
}

struct WhirlApp {
  s1: String,
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
      s1: "Instrument Sans | Editing {address}".to_string(),
    }
  }
}

impl eframe::App for WhirlApp {
  fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
    egui::CentralPanel::default().show(ui, |ui| {
      ui.heading("Hello World!");
      ui.label("Whirling Press 飞旋印刷厂");
      ui.add(egui::TextEdit::singleline(&mut self.s1).hint_text("Write something here"));
    });
  }
}
