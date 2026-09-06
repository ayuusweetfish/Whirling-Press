use eframe::egui;

pub(crate) fn start(root_path: Option<&str>) {
  let _ = eframe::run_native(
    "Whirling Press",
    eframe::NativeOptions::default(),
    Box::new(|cc| Ok(Box::new(WhirlApp::new(cc, root_path)))),
  );
}

use crate::model;

struct WhirlApp {
  root_path: Option<std::path::PathBuf>,
  attr_panel_expanded: bool,
  layout: Option<model::Layout>,
  font_families: Vec<String>,
  sel_element: Option<usize>,
}

impl WhirlApp {
  fn new(cc: &eframe::CreationContext<'_>, root_path: Option<&str>) -> Self {
    let ctx = &cc.egui_ctx;

    use egui::{FontDefinitions, FontData, FontFamily, FontId};
    use egui::style::TextStyle;

    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert("f1".to_string(),
      std::sync::Arc::new(FontData::from_static(include_bytes!("../res/InstrumentSans_Regular.ttf"))));
    fonts.font_data.insert("f2".to_string(),
      std::sync::Arc::new(FontData::from_static(include_bytes!("../res/NotoEmoji_Regular_subset.ttf"))));
    fonts.font_data.insert("f9".to_string(),
      std::sync::Arc::new(FontData::from_static(include_bytes!("../res/DroidSansFallbackFull.ttf"))));
    fonts.families.get_mut(&FontFamily::Proportional).unwrap().push("f1".to_owned());
    fonts.families.get_mut(&FontFamily::Proportional).unwrap().push("f2".to_owned());
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

    ctx.all_styles_mut(|style| {
      style.spacing.interact_size = [40., 24.].into();
      style.spacing.combo_height = 300.;
    });

    let mut self_ = Self {
      root_path: None,
      attr_panel_expanded: true,
      layout: None,
      font_families: vec!["Instrument Sans".to_string(), "Noto Sans CJK SC".to_string(), "Droid Sans Fallback".to_string()],
      sel_element: None,
    };

    if let Some(root_path) = root_path {
      self_.load_document(root_path, ctx);
    }

    self_
  }

  fn load_document<P: AsRef<std::path::Path>>(&mut self, root_path: P, ctx: &egui::Context) {
    let root_path = root_path.as_ref();
    match crate::process_pack_metadata(root_path) {
      Ok((_, layout, _, fields)) => {
        self.root_path = Some(root_path.to_path_buf());
        self.layout = Some(layout);
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(
          format!("{} — Whirling Press", root_path.display())));
      },
      Err(e) => {
        println!("{:?}", e);
      },
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
          self.load_document(&path, ui.ctx());
        }
        ui.add_space(6.0);
        ui.label("or drag-and-drop");
      });
    });
  }

  fn ui_edit(&mut self, ui: &mut egui::Ui) {
    use egui::{Layout, Align, Grid};

    egui::Panel::right("attr_panel")
      .resizable(true).default_size(320.0).size_range(80.0..=800.0)
      .show_collapsible(ui, &mut self.attr_panel_expanded, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
          ui.with_layout(Layout::top_down_justified(Align::Min), |ui| {
            if let Some(i) = self.sel_element {
              let el = &mut self.layout.as_mut().unwrap().elements[i];
              ui.heading(match &el.content {
                model::Content::Image { .. } => "Image",
                model::Content::Text { .. } => "Text",
              });
              Grid::new("attr_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                  Self::show_attrs(ui, &self.font_families, el);
                });
            } else {
              ui.label("");
            }
          });
        });
      });
    egui::CentralPanel::no_frame()
      .frame(egui::Frame::canvas(ui.style()))
      .show(ui, |ui| {
        ui.with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
          egui::CollapsingHeader::new("Elements").default_open(true).show(ui, |ui| {
            let h = ui.style().spacing.interact_size.y;
            ui.allocate_ui(egui::vec2(160.0 + (h + ui.style().spacing.item_spacing.x) * 2.0, 240.0), |ui| {
              egui::ScrollArea::vertical().show(ui, |ui| {
                let elements = &mut self.layout.as_mut().unwrap().elements;
                // Button for adding new
                let add_resp = ui.add_sized(
                  [ui.available_width(), ui.style().spacing.interact_size.y],
                  egui::Button::selectable(false, "+")
                ).on_hover_text("Add");
                egui::Popup::menu(&add_resp).id(ui.id().with("add_element_popup"))
                  .show(|ui| {
                    ui.set_min_width(add_resp.rect.width() -
                      ui.style().spacing.menu_margin.left as f32 -
                      ui.style().spacing.menu_margin.right as f32);
                    if ui.add(egui::Button::selectable(false, "🖼️ Image")).clicked() {
                      elements.push(model::Element::from_content(model::Content::Image {
                        src: "".to_string(),
                      }));
                    }
                    if ui.add(egui::Button::selectable(false, "💬 Text")).clicked() {
                      elements.push(model::Element::from_content(model::Content::Text {
                        family: vec![self.font_families[0].clone()],
                        size: 16.0,
                        weight: model::Weight::Regular,
                        italic: false,
                        tracking: 0.0,
                        text: "".to_string(),
                      }));
                    }
                  });
                // List
                let mut to_prioritize = None;
                let mut to_remove = None;
                for (i, el) in elements.iter().enumerate().rev() {
                  let sel = self.sel_element.map_or(false, |j| j == i);
                  ui.horizontal(|ui| {
                    if ui.add_sized(
                      egui::vec2(160.0, ui.style().spacing.interact_size.y),
                      egui::Button::selectable(sel, "").left_text(
                        match &el.content {
                          model::Content::Image { src, .. } => format!("🖼️ {}", src),
                          model::Content::Text { text, .. } => format!("💬 {}", text),
                        },
                      ).truncate(),
                    ).clicked() {
                      self.sel_element = Some(i);
                    }
                    if i + 1 == elements.len() {
                      ui.add_space(h + ui.style().spacing.item_spacing.x);
                    } else {
                      if ui.add_sized([h, h], egui::Button::new("↑")).on_hover_text("Move up").clicked() {
                        to_prioritize = Some(i);
                      }
                    }
                    if ui.add_sized([h, h], egui::Button::new("×")).on_hover_text("Remove").clicked() {
                      to_remove = Some(i);
                    }
                  });
                }
                if let Some(i) = to_prioritize {
                  elements.swap(i, i + 1);
                  match self.sel_element {
                    Some(j) if j == i => self.sel_element = Some(i + 1),
                    Some(j) if j == i + 1 => self.sel_element = Some(i),
                    _ => (),
                  }
                }
                if let Some(i) = to_remove {
                  elements.remove(i);
                  match self.sel_element {
                    Some(j) if j == i => self.sel_element = None,
                    Some(j) if j > i => self.sel_element = Some(j - 1),
                    _ => (),
                  }
                }
              });
            });
          });
        });
      });
  }

  fn show_attrs(ui: &mut egui::Ui, font_families: &[String], el: &mut model::Element) {
    use egui::{DragValue, Button, ComboBox};
    ui.label("Position");
    ui.horizontal(|ui| {
      ui.add_sized([80.0, ui.style().spacing.interact_size.y], DragValue::new(&mut el.x).speed(0.1));
      ui.add_sized([80.0, ui.style().spacing.interact_size.y], DragValue::new(&mut el.y).speed(0.1));
      ui.add_space(ui.available_width()); // XXX: Hack to extend the grid to fill remaining width?
    });
    ui.end_row();
    ui.label("Anchor");
    let anchors_and_names = [
      (model::Anchor { x: model::AnchorAxis::Min, y: model::AnchorAxis::Min }, "Top Left"),
      (model::Anchor { x: model::AnchorAxis::Min, y: model::AnchorAxis::Cen }, "Top Center"),
      (model::Anchor { x: model::AnchorAxis::Min, y: model::AnchorAxis::Max }, "Top Right"),
      (model::Anchor { x: model::AnchorAxis::Cen, y: model::AnchorAxis::Min }, "Middle Left"),
      (model::Anchor { x: model::AnchorAxis::Cen, y: model::AnchorAxis::Cen }, "Middle Center"),
      (model::Anchor { x: model::AnchorAxis::Cen, y: model::AnchorAxis::Max }, "Middle Right"),
      (model::Anchor { x: model::AnchorAxis::Max, y: model::AnchorAxis::Min }, "Bottom Left"),
      (model::Anchor { x: model::AnchorAxis::Max, y: model::AnchorAxis::Cen }, "Bottom Center"),
      (model::Anchor { x: model::AnchorAxis::Max, y: model::AnchorAxis::Max }, "Bottom Right"),
    ];
    ComboBox::from_id_salt("element_anchor")
      .width(150.0)
      .selected_text(anchors_and_names.iter()
        .find(|(a, _)| *a == el.anchor)
        .map(|(_, name)| *name).unwrap())
      .show_ui(ui, |ui| {
        for (value, name) in anchors_and_names {
          ui.selectable_value(&mut el.anchor, value, name);
        }
      });
    ui.end_row();
    ui.label("Scale");
    ui.horizontal(|ui| {
      ui.add_sized([80.0, ui.style().spacing.interact_size.y], DragValue::new(&mut el.scale.0).speed(0.01));
      ui.add_sized([80.0, ui.style().spacing.interact_size.y], DragValue::new(&mut el.scale.1).speed(0.01));
    });
    ui.end_row();
    ui.label("Rotation");
    ui.horizontal(|ui| {
      ui.add_sized([80.0, ui.available_height()], DragValue::new(&mut el.rotation).speed(0.1));
    });
    ui.end_row();
    ui.label("Size limit");
    fn size_limit_editor(ui: &mut egui::Ui, value: &mut Option<f32>) {
      let mut x = value.unwrap_or(0.0);
      ui.add_sized([80.0, ui.style().spacing.interact_size.y],
        DragValue::new(&mut x).range(0.0..=f32::INFINITY).speed(0.1)
          .custom_formatter(|n, _| {
            if n > 0.0 { format!("{n:.1}") } else { "—".to_string() }
          })
          .custom_parser(|s| {
            let s = s.trim();
            if s == "" || s.starts_with("—") || s.starts_with("-") {
              Some(0.0)
            } else {
              s.parse().ok()
            }
          }));
      *value = if x > 0.0 { Some(x) } else { None };
    }
    ui.horizontal(|ui| {
      size_limit_editor(ui, &mut el.width);
      size_limit_editor(ui, &mut el.height);
    });
    ui.end_row();
    match &mut el.content {
      model::Content::Image { src } => {
        ui.label("Source");
        ui.add(egui::TextEdit::singleline(src));
        ui.end_row();
      },
      model::Content::Text { family, size, weight, italic, tracking, text } => {
        ui.label("Text");
        ui.add(egui::TextEdit::multiline(text).desired_width(ui.available_width()));
        ui.end_row();
        ui.label("Family");
        Self::font_list_editor(ui, &font_families, family);
        ui.end_row();
        ui.label("Size");
        ui.add_sized([80.0, ui.style().spacing.interact_size.y], DragValue::new(size).range(0.0..=f32::INFINITY).speed(1.0));
        ui.end_row();
        ui.label("Weight");
        let weights_and_names = [
          (model::Weight::Thin, "Thin (100)"),
          (model::Weight::ExtraLight, "Extra Light (200)"),
          (model::Weight::Light, "Light (300)"),
          (model::Weight::Regular, "Regular (400)"),
          (model::Weight::Medium, "Medium (500)"),
          (model::Weight::SemiBold, "Semi Bold (600)"),
          (model::Weight::Bold, "Bold (700)"),
          (model::Weight::ExtraBold, "Extra Bold (800)"),
          (model::Weight::Black, "Black (900)"),
        ];
        ComboBox::from_id_salt("font_weight")
          .width(150.0)
          .selected_text(weights_and_names.iter()
            .find(|(a, _)| *a == *weight)
            .map(|(_, name)| *name).unwrap())
          .show_ui(ui, |ui| {
            for (value, name) in weights_and_names {
              ui.selectable_value(weight, value, name);
            }
          });
        ui.end_row();
        ui.label("Italic");
        ui.checkbox(italic, "");
        ui.end_row();
        ui.label("Tracking");
        ui.add_sized([80.0, ui.style().spacing.interact_size.y], DragValue::new(tracking).speed(0.1));
        ui.end_row();
      },
    }
  }

  fn font_list_editor(ui: &mut egui::Ui, font_families: &[String], el: &mut Vec<String>) {
    ui.vertical(|ui| {
      let mut to_prioritize = None;
      let mut to_remove = None;
      let n = el.len();
      for (i, s) in el.iter_mut().enumerate() {
        // ui.dnd_drag_source(ui.id().with(("font_list", i)), i, |ui| {
        ui.horizontal(|ui| {
          let h = ui.available_height();
          let button_w = if n > 1 {
            ui.available_width() - (h + ui.style().spacing.item_spacing.x) * 2.0
          } else {
            ui.available_width()
          };
          let resp = ui.add_sized([button_w, h],
            egui::Button::selectable(false, &*s));
          egui::Popup::menu(&resp).id(ui.id().with(("font_family_popup", i)))
            .show(|ui| {
              ui.set_min_width(resp.rect.width() -
                ui.style().spacing.menu_margin.left as f32 -
                ui.style().spacing.menu_margin.right as f32);
              egui::ScrollArea::vertical()
                .max_height(ui.style().spacing.combo_height)
                .show(ui, |ui| {
                  ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                  for family in font_families {
                    if ui.add(egui::Button::selectable(*family == *s, family)).clicked() {
                      *s = family.clone();
                    }
                  }
                }).inner
            });
          if n > 1 {
            if i == 0 {
              ui.add_space(h + ui.style().spacing.item_spacing.x);
            } else {
              if ui.add_sized([h, h], egui::Button::new("↑")).on_hover_text("Move up").clicked() {
                to_prioritize = Some(i);
              }
            }
            if ui.add_sized([h, h], egui::Button::new("×")).on_hover_text("Remove").clicked() {
              to_remove = Some(i);
            }
          }
        });
        // });
      }
      if let Some(i) = to_prioritize {
        el.swap(i, i - 1);
      }
      if let Some(i) = to_remove {
        el.remove(i);
      }
      if ui.add_sized([ui.available_width(), 0.0],
        egui::Button::selectable(false, "+")).on_hover_text("Add").clicked()
      {
        el.push(font_families[0].clone());
      }
    });
  }
}

impl eframe::App for WhirlApp {
  fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
    egui::Panel::bottom("status_bar")
      .frame(egui::Frame::side_top_panel(ui.style()))
      .show(ui, |ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
          // `egui::widgets::global_theme_preference_switch` uses a frameless button
          // which has less-than-ideal hover feedback
          let is_dark = ui.ctx().theme() == egui::Theme::Dark;
          if ui.add_sized([ui.available_height(), ui.available_height()],
            egui::Button::selectable(false,
              egui::RichText::new(if is_dark { "🌙" } else { "☀️" }).size(14.0))).clicked() {
            ui.ctx().set_theme(if is_dark { egui::Theme::Light } else { egui::Theme::Dark });
          }
          ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            if let Some(path) = &self.root_path {
              if ui.add_sized([ui.available_height(), ui.available_height()],
                egui::Button::selectable(false, egui::RichText::new("📄").size(14.0))).clicked() {
              }
              ui.label(format!("759×2663, 25 elements — {}", path.to_string_lossy()));
            }
          });
        });
      });
    if let Some(_) = &self.root_path {
      self.ui_edit(ui);
    } else {
      self.ui_landing(ui);
    }
  }
}
