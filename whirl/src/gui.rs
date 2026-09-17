use eframe::egui;

pub(crate) fn start(root_path: Option<&str>) {
  let result = eframe::run_native(
    "Whirling Press",
    eframe::NativeOptions::default(),
    Box::new(|cc| Ok(Box::new(WhirlApp::new(cc, root_path)))),
  );
  if let Err(e) = result {
    eprintln!("Error: {}", e);
  }
}

use crate::model;
use crate::render;

struct WhirlApp {
  root_path: Option<std::path::PathBuf>,
  attr_panel_expanded: bool,
  error_message: Option<(bool, String)>,
  layout: Option<model::Layout>,
  font_families: Vec<String>,
  just_loaded: bool,
  sel_element: Option<usize>,
  element_texs: Vec<egui::TextureHandle>,
  canvas: CanvasState,
}

struct CanvasState {
  pub zoom: f32,
  pub origin: (f32, f32),
}
impl Default for CanvasState {
  fn default() -> Self {
    Self {
      zoom: 1.0,
      origin: (0.0, 0.0),
    }
  }
}

const RENDER_TEX_MARGIN: f32 = 32.0;
const RENDER_TEX_SCALE: f32 = 2.0;

fn element_to_image<P: AsRef<std::path::Path>>(root_path: P, el: &model::Element) -> egui::ColorImage {
  use crate::{ProcessError, escape};
  fn try_to_image<P: AsRef<std::path::Path>>(root_path: P, src: &str) -> Result<egui::ColorImage, ProcessError> {
    let src = include_str!("support.typ").to_string() +
  &format!(r#"
#set page(width: auto, height: auto, fill: none, margin: {}pt)
  "#, RENDER_TEX_MARGIN) + src;
    let buf = render::to_rgba(root_path, &src, RENDER_TEX_SCALE)
      .map_err(|e| ProcessError::Render(e))?;
    Ok(egui::ColorImage::from_rgba_premultiplied([buf.width as usize, buf.height as usize], &buf.pixels))
  }
  match try_to_image(
    &root_path,
    &el.typst_source(&[], true).unwrap(),
  ) {
    Ok(image) => image,
    Err(e) => {
      // XXX: Display better error messages
      try_to_image(root_path,
        &format!("#place-element(0pt, 0pt, in_flow: true)[#raw(\"{}\")]", escape(&format!("{}", e))),
      ).unwrap()
    },
  }
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
    fonts.font_data.insert("f3".to_string(),
      std::sync::Arc::new(FontData::from_static(include_bytes!("../res/NotoSansMono_Condensed_Regular_subset.ttf"))));
    fonts.font_data.insert("f9".to_string(),
      std::sync::Arc::new(FontData::from_static(include_bytes!("../res/DroidSansFallbackFull.ttf"))));
    fonts.families.get_mut(&FontFamily::Proportional).unwrap().push("f1".to_owned());
    fonts.families.get_mut(&FontFamily::Proportional).unwrap().push("f2".to_owned());
    fonts.families.get_mut(&FontFamily::Proportional).unwrap().push("f3".to_owned());
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
      error_message: None,
      layout: None,
      font_families: crate::render::font_families(None::<&str>),
      just_loaded: false,
      sel_element: None,
      element_texs: vec![],
      canvas: CanvasState::default(),
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
        self.font_families = crate::render::font_families(Some(root_path));
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(
          format!("{} — Whirling Press", root_path.display())));

        self.just_loaded = true;
        self.element_texs = layout.elements.iter().map(|el| {
          ctx.load_texture(
            "",
            element_to_image(root_path, &el),
            egui::TextureOptions::default(),
          )
        }).collect();

        self.layout = Some(layout);

        self.error_message = None;
      },
      Err(e) => {
        self.error_message = Some((true, e.to_string()));
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
        if ui.button("Open content folder").clicked() {
          self.error_message = None;
          if let Some(path) = rfd::FileDialog::new().pick_folder() {
            self.load_document(&path, ui.ctx());
          }
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
              ui.push_id(
                i,
                |ui| {
                  Grid::new(("attr_grid", i))
                    .num_columns(2)
                    .spacing([20.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                      if Self::show_attrs(ui, &self.font_families, el) {
                        self.element_texs[i] =
                          ui.ctx().load_texture(
                            "",
                            element_to_image(self.root_path.as_ref().unwrap(), el),
                            egui::TextureOptions::default(),
                          );
                      }
                    });
                });
            } else {
              ui.label("");
            }
          });
        });
      });

    // Both central and canvas
    let (c_width, c_height) = ui.available_size().into();

    egui::CentralPanel::no_frame()
      .frame(egui::Frame::canvas(ui.style()).stroke(egui::Stroke::NONE).corner_radius(0.0))
      .show(ui, |ui| {
        // Painter
        let (resp, painter) = ui.allocate_painter(
          ui.available_size_before_wrap(),
          egui::Sense::click_and_drag()
        );

        if ui.input(|i|
          i.multi_touch().is_some() ||
          i.events.iter().any(|event| matches!(event, egui::Event::Zoom { .. })) ||
          (resp.contains_pointer() &&
           i.events.iter().any(|event| matches!(event, egui::Event::MouseWheel { .. })))
        ) {
          ui.input(|i| {
            self.canvas.zoom *= i.zoom_delta();
            self.canvas.origin = (
              self.canvas.origin.0 - i.translation_delta().x / self.canvas.zoom,
              self.canvas.origin.1 - i.translation_delta().y / self.canvas.zoom,
            );
          });
        } else if resp.dragged() {
          self.canvas.origin = (
            self.canvas.origin.0 - resp.drag_delta().x / self.canvas.zoom,
            self.canvas.origin.1 - resp.drag_delta().y / self.canvas.zoom,
          );
        }

        let (page_w, page_h) = self.layout.as_ref().unwrap().page;
        if self.just_loaded {
          self.just_loaded = false;
          self.canvas.zoom =
            ((c_width - 24.0).max(24.0) / page_w)
            .min((c_height - 24.0).max(24.0) / page_h);
        }

        let doc_to_canvas = |(x, y): (f32, f32)| -> (f32, f32) {
          (
            c_width / 2.0 + (x - (self.canvas.origin.0 + page_w / 2.0)) * self.canvas.zoom,
            c_height / 2.0 + (y - (self.canvas.origin.1 + page_h / 2.0)) * self.canvas.zoom,
          )
        };
        let canvas_to_doc = |(cx, cy): (f32, f32)| -> (f32, f32) {
          (
            (cx - c_width / 2.0) / self.canvas.zoom + self.canvas.origin.0 + page_w / 2.0,
            (cy - c_height / 2.0) / self.canvas.zoom + self.canvas.origin.1 + page_h / 2.0,
          )
        };

        // Hover position in canvas coordinates
        let hover_pos: Option<(f32, f32)> = resp.hover_pos().map(|p| p.into());

        painter.add(egui::Shape::rect_filled(
          egui::Rect::from_min_size(
            doc_to_canvas((0., 0.)).into(),
            [page_w * self.canvas.zoom, page_h * self.canvas.zoom].into()),
          egui::CornerRadius::same(5),
          egui::Color32::from_rgba_premultiplied(255, 248, 240, 255),
        ));

        // Transforms a point in a rectangular region (texture)
        // with a pinned anchor into world coordinates
        fn transformed_anchored(
          (x0, y0): (f32, f32),   // Anchor is pinned here
          (ax, ay): (f32, f32),   // Anchor inside the texture (in original units)
          (sx, sy): (f32, f32),   // Scale
          rota: f32,              // Rotation (radians)
          (tx, ty): (f32, f32),   // Point in question inside texture (in original units)
        ) -> (f32, f32) {
          let dx = tx - ax;
          let dy = ty - ay;
          let sdx = dx * sx;
          let sdy = dy * sy;
          let (sin, cos) = rota.sin_cos();
          let rx = sdx * cos - sdy * sin;
          let ry = sdx * sin + sdy * cos;
          (x0 + rx, y0 + ry)
        }

        fn element_tex_pos(el: &model::Element, (w, h): (f32, f32), (tx, ty): (f32, f32)) -> (f32, f32) {
          transformed_anchored(
            (el.x, el.y),
            (RENDER_TEX_MARGIN * RENDER_TEX_SCALE + (w - RENDER_TEX_MARGIN * RENDER_TEX_SCALE * 2.) * f32::from(el.anchor.x),
             RENDER_TEX_MARGIN * RENDER_TEX_SCALE + (h - RENDER_TEX_MARGIN * RENDER_TEX_SCALE * 2.) * f32::from(el.anchor.y)),
            (el.scale.0 / RENDER_TEX_SCALE, el.scale.1 / RENDER_TEX_SCALE),
            el.rotation,
            (tx * w, ty * h),
          )
        }
        fn element_tex_pos_content(el: &model::Element, (w, h): (f32, f32), (tx, ty): (f32, f32)) -> (f32, f32) {
          transformed_anchored(
            (el.x, el.y),
            (RENDER_TEX_MARGIN * RENDER_TEX_SCALE + (w - RENDER_TEX_MARGIN * RENDER_TEX_SCALE * 2.) * f32::from(el.anchor.x),
             RENDER_TEX_MARGIN * RENDER_TEX_SCALE + (h - RENDER_TEX_MARGIN * RENDER_TEX_SCALE * 2.) * f32::from(el.anchor.y)),
            (el.scale.0 / RENDER_TEX_SCALE, el.scale.1 / RENDER_TEX_SCALE),
            el.rotation,
            (RENDER_TEX_MARGIN * RENDER_TEX_SCALE + tx * (w - RENDER_TEX_MARGIN * RENDER_TEX_SCALE * 2.),
             RENDER_TEX_MARGIN * RENDER_TEX_SCALE + ty * (h - RENDER_TEX_MARGIN * RENDER_TEX_SCALE * 2.)),
          )
        }

        fn textured_quad(
          corners: [egui::Pos2; 4], // [00, 10, 11, 01]
          texture_id: egui::TextureId,
        ) -> egui::Shape {
          let mut mesh = egui::Mesh::with_texture(texture_id);
          let uvs = [
            egui::Pos2::new(0.0, 0.0),
            egui::Pos2::new(1.0, 0.0),
            egui::Pos2::new(1.0, 1.0),
            egui::Pos2::new(0.0, 1.0),
          ];
          mesh.vertices.extend(
            corners.iter().zip(uvs.iter()).map(|(pos, uv)| egui::epaint::Vertex {
              pos: *pos,
              uv: *uv,
              color: egui::Color32::WHITE,
            }),
          );
          mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
          egui::Shape::mesh(mesh)
        }

        pub fn point_in_simple_polygon(p: (f32, f32), a: &[(f32, f32)]) -> bool {
          // Even-odd crossing number; optimization by W. Randolph Franklin
          // https://wrf.ecse.rpi.edu/Research/Short_Notes/pnpoly.html
          let (x0, y0) = p;
          let mut c = false;
          for i in 0..a.len() {
            let (x1, y1) = a[i];
            let (x2, y2) = a[(i + 1) % a.len()];
            if ((y2 > y0) != (y1 > y0)) &&
               x0 < (x1 - x2) * (y0 - y2) / (y1 - y2) + x2 {
              c = !c;
            }
          }
          c
        }

        for (i, el) in self.layout.as_ref().unwrap().elements.iter().enumerate() {
          let tex = &self.element_texs[i];
          let corners = [
            doc_to_canvas(element_tex_pos(&el, tex.size_vec2().into(), (0., 0.))).into(),
            doc_to_canvas(element_tex_pos(&el, tex.size_vec2().into(), (1., 0.))).into(),
            doc_to_canvas(element_tex_pos(&el, tex.size_vec2().into(), (1., 1.))).into(),
            doc_to_canvas(element_tex_pos(&el, tex.size_vec2().into(), (0., 1.))).into(),
          ];
          painter.add(textured_quad(corners, tex.id()));

          let r = (4.0 * self.canvas.zoom).clamp(2.0, 4.0);
          let corners_content = [
            doc_to_canvas(element_tex_pos_content(&el, tex.size_vec2().into(), (0., 0.))),
            doc_to_canvas(element_tex_pos_content(&el, tex.size_vec2().into(), (1., 0.))),
            doc_to_canvas(element_tex_pos_content(&el, tex.size_vec2().into(), (1., 1.))),
            doc_to_canvas(element_tex_pos_content(&el, tex.size_vec2().into(), (0., 1.))),
          ];
          let mut inside = false;
          if let Some(p) = hover_pos && point_in_simple_polygon(p, &corners_content) {
            inside = true;
          }
          if self.sel_element == Some(i) {
            inside = true;
          }
          if inside {
            painter.line(
              vec![
                corners_content[0].into(), corners_content[1].into(),
                corners_content[2].into(), corners_content[3].into(),
                corners_content[0].into()
              ],
              egui::Stroke::new(r * 0.5, egui::Color32::from_rgba_premultiplied(24, 24, 24, 255)),
            );
          }
          if resp.clicked() {
            if let Some(p) = resp.interact_pointer_pos()
              && point_in_simple_polygon(p.into(), &corners_content)
            {
              self.sel_element = Some(i);
            }
          }
          if self.sel_element == Some(i) {
            painter.add(egui::Shape::Circle(egui::epaint::CircleShape {
              center: doc_to_canvas((el.x, el.y)).into(),
              radius: r,
              fill: egui::Color32::from_rgba_premultiplied(128, 120, 96, 128),
              stroke: egui::Stroke::new(r * 0.5, egui::Color32::from_rgba_premultiplied(24, 24, 24, 255)),
            }));
          }
        }

        // Elements exapndable menu
        egui::Area::new(ui.id().with("elements_area"))
          .fixed_pos(egui::pos2(0.0, 0.0))
          .pivot(egui::Align2::LEFT_TOP)
          .show(ui.ctx(), |ui| {
            egui::Frame::menu(ui.style()).inner_margin(2.0).show(ui, |ui| {
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
                          self.element_texs.push(
                            ui.ctx().load_texture(
                              "",
                              element_to_image(self.root_path.as_ref().unwrap(), &elements.last().unwrap()),
                              egui::TextureOptions::default(),
                            )
                          );
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
                          self.element_texs.push(
                            ui.ctx().load_texture(
                              "",
                              element_to_image(self.root_path.as_ref().unwrap(), &elements.last().unwrap()),
                              egui::TextureOptions::default(),
                            )
                          );
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
                      self.element_texs.swap(i, i + 1);
                      match self.sel_element {
                        Some(j) if j == i => self.sel_element = Some(i + 1),
                        Some(j) if j == i + 1 => self.sel_element = Some(i),
                        _ => (),
                      }
                    }
                    if let Some(i) = to_remove {
                      elements.remove(i);
                      drop(self.element_texs.remove(i));
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

        // Panel toggle
        egui::Area::new(ui.id().with("attr_panel_toggle"))
          .fixed_pos(egui::pos2(c_width, c_height / 2.0))
          .pivot(egui::Align2::RIGHT_CENTER)
          .show(ui.ctx(), |ui| {
            let h = ui.style().spacing.interact_size.y;
            if ui.add(egui::Button::new(if self.attr_panel_expanded { "▸" } else { "◂" }))
              .on_hover_text(if self.attr_panel_expanded { "Hide attributes" } else { "Show attributes" })
              .clicked()
            {
              self.attr_panel_expanded = !self.attr_panel_expanded;
            }
          });
      });
  }

  fn show_attrs(ui: &mut egui::Ui, font_families: &[String], el: &mut model::Element) -> bool {
    let mut changed = false;
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
      (model::Anchor { x: model::AnchorAxis::Min, y: model::AnchorAxis::Min }, "Top Left", "TL"),
      (model::Anchor { x: model::AnchorAxis::Cen, y: model::AnchorAxis::Min }, "Top Center", "TC"),
      (model::Anchor { x: model::AnchorAxis::Max, y: model::AnchorAxis::Min }, "Top Right", "TR"),
      (model::Anchor { x: model::AnchorAxis::Min, y: model::AnchorAxis::Cen }, "Middle Left", "ML"),
      (model::Anchor { x: model::AnchorAxis::Cen, y: model::AnchorAxis::Cen }, "Middle Center", "MC"),
      (model::Anchor { x: model::AnchorAxis::Max, y: model::AnchorAxis::Cen }, "Middle Right", "MR"),
      (model::Anchor { x: model::AnchorAxis::Min, y: model::AnchorAxis::Max }, "Bottom Left", "BL"),
      (model::Anchor { x: model::AnchorAxis::Cen, y: model::AnchorAxis::Max }, "Bottom Center", "BC"),
      (model::Anchor { x: model::AnchorAxis::Max, y: model::AnchorAxis::Max }, "Bottom Right", "BR"),
    ];
    ui.ctx().all_styles_mut(|style| {
      style.spacing.combo_height = 300.;
    });
    ComboBox::from_id_salt("element_anchor")
      .width(150.0)
      .selected_text(anchors_and_names.iter()
        .find(|(a, _, _)| *a == el.anchor)
        .map(|(_, name, _)| *name).unwrap())
      .show_ui(ui, |ui| {
        for i in 0..3 {
          ui.horizontal(|ui| {
            for j in (i * 3)..(i * 3 + 3) {
              let (value, name, abbr) = anchors_and_names[j];
              // From `egui::Ui::selectable_value`
              let (current_value, selected_value, text) = (&mut el.anchor, value, abbr);
              let checked = *current_value == selected_value;
              let mut response =
                ui.add_sized((40., 40.), egui::Button::selectable(checked, text));
              if response.clicked() && *current_value != selected_value {
                *current_value = selected_value;
                response.mark_changed();
              }
            }
          });
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
    fn size_limit_editor(ui: &mut egui::Ui, value: &mut Option<f32>) -> bool {
      let mut x = value.unwrap_or(0.0);
      let resp = ui.add_sized([80.0, ui.style().spacing.interact_size.y],
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
      resp.changed()
    }
    ui.horizontal(|ui| {
      changed |= size_limit_editor(ui, &mut el.width);
      changed |= size_limit_editor(ui, &mut el.height);
    });
    ui.end_row();
    match &mut el.content {
      model::Content::Image { src } => {
        ui.label("Source");
        changed |= ui.add(egui::TextEdit::singleline(src)).changed();
        ui.end_row();
      },
      model::Content::Text { family, size, weight, italic, tracking, text } => {
        ui.label("Text");
        changed |= ui.add(egui::TextEdit::multiline(text).desired_width(ui.available_width())).changed();
        ui.end_row();
        ui.label("Family");
        changed |= Self::font_list_editor(ui, &font_families, family);
        ui.end_row();
        ui.label("Size");
        changed |= ui.add_sized([80.0, ui.style().spacing.interact_size.y], DragValue::new(size).range(0.0..=f32::INFINITY).speed(1.0)).changed();
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
              if ui.selectable_value(weight, value, name).clicked() {
                changed = true;
              }
            }
          });
        ui.end_row();
        ui.label("Italic");
        changed |= ui.checkbox(italic, "").changed();
        ui.end_row();
        ui.label("Tracking");
        changed |= ui.add_sized([80.0, ui.style().spacing.interact_size.y], DragValue::new(tracking).speed(0.1)).changed();
        ui.end_row();
      },
    }
    changed
  }

  fn font_list_editor(ui: &mut egui::Ui, font_families: &[String], el: &mut Vec<String>) -> bool {
    let mut changed = false;
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
            egui::Button::selectable(false, &*s).wrap_mode(egui::TextWrapMode::Truncate));
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
                      changed = true;
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
        changed = true;
      }
      if let Some(i) = to_remove {
        el.remove(i);
        changed = true;
      }
      if ui.add_sized([ui.available_width(), 0.0],
        egui::Button::selectable(false, "+")).on_hover_text("Add").clicked()
      {
        el.push(font_families[0].clone());
        changed = true;
      }
    });
    changed
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
              let layout = self.layout.as_mut().unwrap();
              let resp = ui.add_sized([ui.available_height(), ui.available_height()],
                egui::Button::selectable(false, egui::RichText::new("📄").size(14.0)));
              egui::Popup::menu(&resp).show(|ui| {
                ui.horizontal(|ui| {
                  ui.label("Page size");
                  ui.add_sized([80.0, ui.style().spacing.interact_size.y], egui::DragValue::new(&mut layout.page.0).speed(1).range(1.0..=f32::INFINITY));
                  ui.add_sized([80.0, ui.style().spacing.interact_size.y], egui::DragValue::new(&mut layout.page.1).speed(1).range(1.0..=f32::INFINITY));
                });
              });
              if let Some((_is_err, s)) = &self.error_message {
                ui.label(format!("Error: {}", s));
              } else {
                ui.label(format!("{}×{}, {} elements — {}",
                  layout.page.0, layout.page.1,
                  layout.elements.len(),
                  path.to_string_lossy()));
              }
            } else if let Some((_is_err, s)) = &self.error_message {
              ui.label(format!("Error: {}", s));
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
