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
use crate::f;   // Fluent localization; src/localize.rs

struct WhirlApp {
  root_path: Option<std::path::PathBuf>,
  attr_panel_expanded: bool,
  status_msg: Option<(bool, std::time::Instant, String)>,
  yml_file_name: Option<std::path::PathBuf>,
  layout: Option<model::Layout>,
  font_families: Vec<String>,
  just_loaded: bool,
  edited: bool,
  sel_element: Option<usize>,
  sel_element_scroll: bool,
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
        &format!("#place-element(0pt, 0pt, in_flow: true)[#raw(\"{}\")]", escape(&format!("{}", e.message()))),
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
      status_msg: None,
      yml_file_name: None,
      layout: None,
      font_families: crate::render::font_families(None::<&str>),
      just_loaded: false,
      edited: false,
      sel_element: None,
      sel_element_scroll: false,
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
    match crate::process_pack_metadata(root_path, true) {
      Ok((yml_file_name, layout, _, _fields)) => {
        self.root_path = Some(root_path.to_path_buf());
        self.yml_file_name = Some(yml_file_name);
        self.font_families = crate::render::font_families(Some(root_path));
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(
          format!("{} — Whirling Press", root_path.display())));

        self.just_loaded = true;
        self.edited = false;
        self.element_texs = layout.elements.iter().map(|el| {
          ctx.load_texture(
            "",
            element_to_image(root_path, &el),
            egui::TextureOptions::default(),
          )
        }).collect();

        self.layout = Some(layout);

        self.status_msg = None;
      },
      Err(e) => {
        self.status_msg = Some((true, std::time::Instant::now(), e.message()));
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
        if ui.button(f!("open-content-folder")).clicked() ||
          ui.input_mut(|i| i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::O)))
        {
          self.status_msg = None;
          if let Some(path) = rfd::FileDialog::new().pick_folder() {
            self.load_document(&path, ui.ctx());
          }
        }
        ui.add_space(6.0);
        ui.label(f!("or-drag-and-drop"));
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
                model::Content::Image { .. } => f!("attr-heading-image"),
                model::Content::Text { .. } => f!("attr-heading-text"),
              });
              ui.push_id(
                i,
                |ui| {
                  Grid::new(("attr_grid", i))
                    .num_columns(2)
                    .spacing([20.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                      let (changed_any, changed_tex) = Self::show_attrs(ui, &self.font_families, el);
                      self.edited |= changed_any;
                      if changed_tex {
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

        // For alternating layers selection on the canvas
        // If previous selection does not contain current pointer, select topmost colliding
        // Otherwise select the next colliding element, bottom-up, cycling
        let mut first_sel: Option<usize> = None;
        let mut last_sel: Option<usize> = None;
        let mut first_above_sel: Option<usize> = None;
        let mut prev_sel_repeated = false;

        for (i, el) in self.layout.as_ref().unwrap().elements.iter().enumerate() {
          // XXX: DRY
          let tex = &self.element_texs[i];
          let corners_content = [
            doc_to_canvas(element_tex_pos_content(&el, tex.size_vec2().into(), (0., 0.))),
            doc_to_canvas(element_tex_pos_content(&el, tex.size_vec2().into(), (1., 0.))),
            doc_to_canvas(element_tex_pos_content(&el, tex.size_vec2().into(), (1., 1.))),
            doc_to_canvas(element_tex_pos_content(&el, tex.size_vec2().into(), (0., 1.))),
          ];
          if resp.clicked() {
            if let Some(p) = resp.interact_pointer_pos()
              && point_in_simple_polygon(p.into(), &corners_content)
            {
              if first_sel.is_none() {
                first_sel = Some(i);
              }
              last_sel = Some(i);
              if let Some(j) = self.sel_element {
                if i == j {
                  prev_sel_repeated = true;
                } else if i > j && first_above_sel.is_none() {
                  first_above_sel = Some(i);
                }
              }
            }
          }
        }

        if first_sel.is_some() {
          self.sel_element = Some(
            if prev_sel_repeated { first_above_sel.unwrap_or(first_sel.unwrap()) }
            else { last_sel.unwrap() }
          );
          self.sel_element_scroll = true;
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
              egui::CollapsingHeader::new(f!("elements")).default_open(true).show(ui, |ui| {
                let h = ui.style().spacing.interact_size.y;
                ui.allocate_ui(egui::vec2(160.0 + (h + ui.style().spacing.item_spacing.x) * 2.0, 240.0), |ui| {
                  egui::ScrollArea::vertical().show(ui, |ui| {
                    let layout = &mut self.layout.as_mut().unwrap();
                    let elements = &mut layout.elements;
                    // Button for adding new
                    let add_resp = ui.add_sized(
                      [ui.available_width(), ui.style().spacing.interact_size.y],
                      egui::Button::selectable(false, "+")
                    ).on_hover_text(f!("list-add"));
                    egui::Popup::menu(&add_resp).id(ui.id().with("add_element_popup"))
                      .show(|ui| {
                        ui.set_min_width(add_resp.rect.width() -
                          ui.style().spacing.menu_margin.left as f32 -
                          ui.style().spacing.menu_margin.right as f32);
                        if ui.add(egui::Button::selectable(false, format!("🖼️ {}", f!("attr-heading-image")))).clicked() {
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
                          self.edited = true;
                        }
                        if ui.add(egui::Button::selectable(false, format!("💬 {}", f!("attr-heading-text")))).clicked() {
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
                          self.edited = true;
                        }
                      });
                    // List
                    let mut to_prioritize = None;
                    let mut to_remove = None;
                    for (i, el) in elements.iter().enumerate().rev() {
                      let sel = self.sel_element.map_or(false, |j| j == i);
                      ui.horizontal(|ui| {
                        let resp = ui.add_sized(
                          egui::vec2(160.0, ui.style().spacing.interact_size.y),
                          egui::Button::selectable(sel, "").left_text(
                            match &el.content {
                              model::Content::Image { src, .. } => format!("🖼️ {}", src),
                              model::Content::Text { text, .. } => format!("💬 {}", text),
                            },
                          ).truncate(),
                        );
                        if sel && self.sel_element_scroll {
                          self.sel_element_scroll = false;
                          resp.scroll_to_me_animation(None,
                            egui::style::ScrollAnimation::new(1500., egui::Rangef::new(0.1, 0.15)));
                        }
                        if resp.clicked() {
                          self.sel_element = Some(i);
                          // Pan canvas to element
                          if el.x - layout.page.0 * 0.5 < self.canvas.origin.0 - c_width * 0.5 / self.canvas.zoom
                            || el.x - layout.page.0 * 0.5 > self.canvas.origin.0 + c_width * 0.5 / self.canvas.zoom
                            || el.y - layout.page.1 * 0.5 < self.canvas.origin.1 - c_height * 0.5 / self.canvas.zoom
                            || el.y - layout.page.1 * 0.5 > self.canvas.origin.1 + c_height * 0.5 / self.canvas.zoom
                          {
                            self.canvas.origin = (
                              el.x - layout.page.0 * 0.5,
                              el.y - layout.page.1 * 0.5,
                            );
                          }
                        }
                        if i + 1 == elements.len() {
                          ui.add_space(h + ui.style().spacing.item_spacing.x);
                        } else {
                          if ui.add_sized([h, h], egui::Button::new("↑")).on_hover_text(f!("list-move-up")).clicked() {
                            to_prioritize = Some(i);
                          }
                        }
                        if ui.add_sized([h, h], egui::Button::new("×")).on_hover_text(f!("list-remove")).clicked() {
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
                      self.edited = true;
                    }
                    if let Some(i) = to_remove {
                      elements.remove(i);
                      drop(self.element_texs.remove(i));
                      match self.sel_element {
                        Some(j) if j == i => self.sel_element = None,
                        Some(j) if j > i => self.sel_element = Some(j - 1),
                        _ => (),
                      }
                      self.edited = true;
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
            if ui.add(egui::Button::new(if self.attr_panel_expanded { "▸" } else { "◂" }))
              .on_hover_text(if self.attr_panel_expanded { "Hide attributes" } else { "Show attributes" })
              .clicked()
            {
              self.attr_panel_expanded = !self.attr_panel_expanded;
            }
          });
      });
  }

  fn show_attrs(ui: &mut egui::Ui, font_families: &[String], el: &mut model::Element) -> (bool, bool) {
    let mut changed_tex = false;
    let mut changed_any = false;
    use egui::{DragValue, ComboBox};
    ui.label(f!("attr-position"));
    ui.horizontal(|ui| {
      changed_any |= ui.add_sized([80.0, ui.style().spacing.interact_size.y], DragValue::new(&mut el.x).speed(0.1)).changed();
      changed_any |= ui.add_sized([80.0, ui.style().spacing.interact_size.y], DragValue::new(&mut el.y).speed(0.1)).changed();
      ui.add_space(ui.available_width()); // XXX: Hack to extend the grid to fill remaining width?
    });
    ui.end_row();
    ui.label(f!("attr-anchor"));
    let anchors_and_names = [
      (model::Anchor { x: model::AnchorAxis::Min, y: model::AnchorAxis::Min }, "anchor-tl", "↖"),
      (model::Anchor { x: model::AnchorAxis::Cen, y: model::AnchorAxis::Min }, "anchor-tc", "↑"),
      (model::Anchor { x: model::AnchorAxis::Max, y: model::AnchorAxis::Min }, "anchor-tr", "↗"),
      (model::Anchor { x: model::AnchorAxis::Min, y: model::AnchorAxis::Cen }, "anchor-ml", "←"),
      (model::Anchor { x: model::AnchorAxis::Cen, y: model::AnchorAxis::Cen }, "anchor-mc", "＋"),
      (model::Anchor { x: model::AnchorAxis::Max, y: model::AnchorAxis::Cen }, "anchor-mr", "→"),
      (model::Anchor { x: model::AnchorAxis::Min, y: model::AnchorAxis::Max }, "anchor-bl", "↙"),
      (model::Anchor { x: model::AnchorAxis::Cen, y: model::AnchorAxis::Max }, "anchor-bc", "↓"),
      (model::Anchor { x: model::AnchorAxis::Max, y: model::AnchorAxis::Max }, "anchor-br", "↘"),
    ];
    ComboBox::from_id_salt("element_anchor")
      .width(150.0)
      .height(f32::INFINITY)
      .selected_text(anchors_and_names.iter()
        .find(|(a, _, _)| *a == el.anchor)
        .map(|(_, name, _)| f!(name)).unwrap())
      .show_ui(ui, |ui| egui::Grid::new("element_anchor_popup")
        .min_row_height(40.)
        .show(ui, |ui| {
          for i in 0..3 {
            for j in (i * 3)..(i * 3 + 3) {
              let (value, name, abbr) = anchors_and_names[j];
              // From `egui::Ui::selectable_value`
              let (current_value, selected_value, text) = (&mut el.anchor, value, abbr);
              let checked = *current_value == selected_value;
              let mut response = ui.centered_and_justified(
                |ui| ui.add(egui::Button::selectable(checked, text)).on_hover_text(f!(name))
              ).inner;
              if response.clicked() && *current_value != selected_value {
                *current_value = selected_value;
                response.mark_changed();
                changed_any = true;
              }
            }
            ui.end_row();
          }
        }));
    ui.end_row();
    ui.label(f!("attr-scale"));
    ui.horizontal(|ui| {
      changed_any |= ui.add_sized([80.0, ui.style().spacing.interact_size.y], DragValue::new(&mut el.scale.0).speed(0.01)).changed();
      changed_any |= ui.add_sized([80.0, ui.style().spacing.interact_size.y], DragValue::new(&mut el.scale.1).speed(0.01)).changed();
    });
    ui.end_row();
    ui.label(f!("attr-rotation"));
    ui.horizontal(|ui| {
      changed_any |= ui.add_sized([80.0, ui.available_height()], DragValue::new(&mut el.rotation).speed(0.1)).changed();
    });
    ui.end_row();
    ui.label(f!("attr-size-limit"));
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
      changed_tex |= size_limit_editor(ui, &mut el.width);
      changed_tex |= size_limit_editor(ui, &mut el.height);
    });
    ui.end_row();
    match &mut el.content {
      model::Content::Image { src } => {
        ui.label(f!("attr-image-source"));
        changed_tex |= ui.add(egui::TextEdit::singleline(src)).changed();
        ui.end_row();
      },
      model::Content::Text { family, size, weight, italic, tracking, text } => {
        ui.label(f!("attr-text-text"));
        changed_tex |= ui.add(egui::TextEdit::multiline(text).desired_width(ui.available_width())).changed();
        ui.end_row();
        ui.label(f!("attr-text-family"));
        changed_tex |= Self::font_list_editor(ui, &font_families, family);
        ui.end_row();
        ui.label(f!("attr-text-size"));
        changed_tex |= ui.add_sized([80.0, ui.style().spacing.interact_size.y], DragValue::new(size).range(0.0..=f32::INFINITY).speed(1.0)).changed();
        ui.end_row();
        ui.label(f!("attr-text-weight"));
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
                changed_tex = true;
              }
            }
          });
        ui.end_row();
        ui.label(f!("attr-text-italic"));
        changed_tex |= ui.checkbox(italic, "").changed();
        ui.end_row();
        ui.label(f!("attr-text-tracking"));
        changed_tex |= ui.add_sized([80.0, ui.style().spacing.interact_size.y], DragValue::new(tracking).speed(0.1)).changed();
        ui.end_row();
      },
    }
    (changed_any || changed_tex, changed_tex)
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
              if ui.add_sized([h, h], egui::Button::new("↑")).on_hover_text(f!("list-move-up")).clicked() {
                to_prioritize = Some(i);
              }
            }
            if ui.add_sized([h, h], egui::Button::new("×")).on_hover_text(f!("list-remove")).clicked() {
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
        egui::Button::selectable(false, "+")).on_hover_text(f!("list-add")).clicked()
      {
        el.push(font_families[0].clone());
        changed = true;
      }
    });
    changed
  }
}

impl eframe::App for WhirlApp {
  fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
    egui::Panel::bottom("status_bar")
      .frame(egui::Frame::side_top_panel(ui.style()))
      .show(ui, |ui| {
        let close_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::W);
        let mut to_close = ui.input_mut(|i| i.consume_shortcut(&close_shortcut));
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
              // Global shortcuts
              let save_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::S);
              let mut to_save = ui.input_mut(|i| i.consume_shortcut(&save_shortcut));
              // Widgets
              let resp = ui.add_sized([ui.available_height(), ui.available_height()],
                egui::Button::selectable(false, egui::RichText::new("📄").size(14.0)));
              let popup_id = resp.id.with("popup");
              egui::Popup::menu(&resp)
              .id(popup_id)
              .close_behavior(egui::containers::PopupCloseBehavior::CloseOnClickOutside)
              .show(|ui| {
                ui.horizontal(|ui| {
                  ui.label(f!("menu-page-size"));
                  ui.add_sized([80.0, ui.style().spacing.interact_size.y], egui::DragValue::new(&mut layout.page.0).speed(1).range(1.0..=f32::INFINITY));
                  ui.add_sized([80.0, ui.style().spacing.interact_size.y], egui::DragValue::new(&mut layout.page.1).speed(1).range(1.0..=f32::INFINITY));
                });
                if ui.add(egui::Button::new(f!("menu-save")).shortcut_text(ui.ctx().format_shortcut(&save_shortcut))).clicked() {
                  // Save YAML
                  to_save = true;
                  egui::Popup::close_id(ui.ctx(), popup_id);
                }
                if ui.add(egui::Button::new(f!("menu-export-pdf"))).clicked() {
                  if let Some(path) = rfd::FileDialog::new()
                    .add_filter("PDF", &["pdf"])
                    .save_file()
                  {
                    match crate::process_and_write(
                      self.root_path.as_ref().unwrap(),
                      Some(layout),
                      &path,
                      crate::ArgsFormat::Pdf,
                      true,
                    ) {
                      Ok(_) => self.status_msg = Some((false, std::time::Instant::now(), f!("status-exported", path => path.to_string_lossy()))),
                      Err(crate::ProcessError::Expr(index, crate::ExprError::NonexistentField(_pos, name))) => {
                        self.sel_element = Some(index);
                        self.sel_element_scroll = true;
                        self.status_msg = Some((true, std::time::Instant::now(), format!("Nonexistent field {}", name)));
                      },
                      Err(e) => self.status_msg = Some((true, std::time::Instant::now(), format!("{}", e.message()))),
                    }
                  }
                  egui::Popup::close_id(ui.ctx(), popup_id);
                }
                if ui.add(egui::Button::new(f!("menu-export-pdf-seq"))).clicked() {
                  if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    match crate::process_and_write(
                      self.root_path.as_ref().unwrap(),
                      Some(layout),
                      &path,
                      crate::ArgsFormat::PdfSeq,
                      true,
                    ) {
                      Ok(_) => self.status_msg = Some((false, std::time::Instant::now(), f!("status-exported", path => path.to_string_lossy()))),
                      Err(crate::ProcessError::Expr(index, crate::ExprError::NonexistentField(_pos, name))) => {
                        self.sel_element = Some(index);
                        self.sel_element_scroll = true;
                        self.status_msg = Some((true, std::time::Instant::now(), format!("Nonexistent field {}", name)));
                      },
                      Err(e) => self.status_msg = Some((true, std::time::Instant::now(), format!("{}", e.message()))),
                    }
                  }
                  egui::Popup::close_id(ui.ctx(), popup_id);
                }
                if ui.add(egui::Button::new(f!("menu-close")).shortcut_text(ui.ctx().format_shortcut(&close_shortcut))).clicked() {
                  to_close = true;
                  egui::Popup::close_id(ui.ctx(), popup_id);
                }
              });
              // Shortcut operations
              if to_save {
                let yml_file_name = self.yml_file_name.as_ref().unwrap();
                let yml_path = self.root_path.as_ref().unwrap().join(yml_file_name);
                let s = serde_saphyr::to_string_with_options(&layout, serde_saphyr::ser_options! {
                  compact_list_indent: true,
                  yaml_12: true,
                }).unwrap();
                match std::fs::write(&yml_path, s) {
                  Ok(_) => self.status_msg = Some((false, std::time::Instant::now(), format!("Saved {}", yml_file_name.to_string_lossy()))),
                  Err(e) => self.status_msg = Some((true, std::time::Instant::now(), format!("{}", e))),
                }
                self.edited = false;
              };
              // Status message
              if let Some((is_err, _, s)) = &self.status_msg {
                if *is_err {
                  ui.colored_label(
                    ui.style().visuals.warn_fg_color,
                    f!("status-error", error => s),
                  );
                } else {
                  ui.label(s.to_string());
                }
              } else {
                ui.label(format!("{}{}×{}, {}— {}",
                  if self.edited { "* " } else { "" },
                  layout.page.0, layout.page.1,
                  f!("status-num-elements", n => layout.elements.len()),
                  path.to_string_lossy()));
              }
            } else if let Some((is_err, _, s)) = &self.status_msg {
              if *is_err {
                ui.colored_label(
                  ui.style().visuals.warn_fg_color,
                  f!("status-error", error => s),
                );
              } else {
                ui.label(s.to_string());
              }
            }
          });
        });
        if to_close {
          self.root_path = None;
          self.status_msg = None;
          self.yml_file_name = None;
          self.layout = None;
          self.font_families.clear();
          self.element_texs.clear();
        }
      });
    if let Some(_) = &self.root_path {
      self.ui_edit(ui);
    } else {
      self.ui_landing(ui);
    }
    if let Some((_, t, _)) = self.status_msg
      && t.elapsed() > std::time::Duration::from_secs(3)
    {
      self.status_msg = None;
    }
  }
}
