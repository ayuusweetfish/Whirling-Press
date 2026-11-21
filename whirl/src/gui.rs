use eframe::egui;

mod dir_watcher;
mod color_picker;

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
use crate::localize;
use crate::f;   // Fluent localization; src/localize.rs

const UNDO_LIMIT: usize = 10000;

struct WhirlApp {
  root_path: Option<std::path::PathBuf>,
  attr_panel_expanded: bool,
  status_msg: Option<(bool, std::time::Instant, String)>,
  yml_file_name: Option<std::path::PathBuf>,
  layout: Option<model::Layout>,
  font_families: Vec<String>,
  font_families_tag: u64,
  csv_file_name: Option<std::path::PathBuf>,
  csv_fields: Vec<String>,
  asset_files: Vec<String>,
  just_loaded: bool,
  saved_at_undo: usize,
  sel_element: Option<usize>,
  sel_element_scroll: bool,
  dragged_element: Option<(usize, (f32, f32), (f32, f32))>,
    // index, starting position, total dragged distance
  element_texs: Vec<SafeTextureHandle>,
  canvas: CanvasState,
  close_confirming: bool,
  quit_after_close: bool,
  text_sel_range: Option<(usize, usize)>,
  attrs_panel_h: f32,
  attrs_panel_drag_offs: f32,
  watcher: Option<dir_watcher::DirWatcher>,
  undo_stack: std::collections::VecDeque<Operation>,
  redo_stack: std::collections::VecDeque<Operation>,
  undo_start_state: UndoStartState,
  showing_about: bool,
  elements_uid: Vec<usize>,
  elements_seq: usize,
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

#[derive(Default)]
pub struct UndoStartState {
  page_size: Option<(f32, f32)>,
  elements: std::collections::HashMap<usize, model::Element>,
  changed_tex: bool,
}

pub enum Operation {
  AddElement { i: usize, element: model::Element },
  RemoveElement { i: usize },
  SwapElements { i: usize, j: usize },
  ChangeElement { i: usize, element: model::Element, retex: bool },
  ChangePageSize { page_size: (f32, f32) },
  #[allow(unused)]
  Sequence(Vec<Operation>),
}

impl WhirlApp {
  // Applies an operation and returns the inverse operation
  fn op_apply(&mut self, ctx: &egui::Context, op: Operation) -> Operation {
    use Operation::*;
    match op {
      AddElement { i, element } => {
        self.element_texs.insert(i,
          load_texture(ctx, element_to_image(self.root_path.as_ref().unwrap(), &element)));
        self.layout.as_mut().unwrap().elements.insert(i, element);
        self.elements_uid.push(self.elements_seq);
        self.elements_seq = self.elements_seq.wrapping_add(1);
        match self.sel_element {
          Some(j) if j >= i => self.sel_element = Some(j + 1),
          _ => (),
        }
        RemoveElement { i }
      },
      RemoveElement { i } => {
        let element = self.layout.as_mut().unwrap().elements.remove(i);
        self.element_texs.remove(i);
        self.elements_uid.remove(i);
        match self.sel_element {
          Some(j) if j == i => self.sel_element = None,
          Some(j) if j > i => self.sel_element = Some(j - 1),
          _ => (),
        }
        AddElement { i, element }
      },
      SwapElements { i, j } => {
        self.layout.as_mut().unwrap().elements.swap(i, j);
        self.element_texs.swap(i, j);
        self.elements_uid.swap(i, j);
        match self.sel_element {
          Some(k) if k == i => self.sel_element = Some(j),
          Some(k) if k == j => self.sel_element = Some(i),
          _ => (),
        }
        SwapElements { i, j }
      },
      ChangeElement { i, element, retex } => {
        if retex {
          self.element_texs[i] = load_texture(ctx, element_to_image(self.root_path.as_ref().unwrap(), &element));
        }
        let old = std::mem::replace(
          &mut self.layout.as_mut().unwrap().elements[i],
          element,
        );
        ChangeElement { i, element: old, retex }
      },
      ChangePageSize { page_size } => {
        let old = self.layout.as_mut().unwrap().page;
        self.layout.as_mut().unwrap().page = page_size;
        ChangePageSize { page_size: old }
      },
      Sequence(seq) => {
        let inv = seq.into_iter().map(|op| self.op_apply(ctx, op)).rev().collect();
        Sequence(inv)
      },
    }
  }

  fn op(&mut self, ctx: &egui::Context, op: Operation) {
    let inv_op = self.op_apply(ctx, op);
    self.undo_stack.push_back(inv_op);
    self.redo_stack.clear();
    if self.undo_stack.len() >= UNDO_LIMIT {
      self.undo_stack.pop_front();
      self.saved_at_undo = match self.saved_at_undo {
        0 => UNDO_LIMIT + 1,
        x => x - 1,
      };
    }
  }

  fn undo(&mut self, ctx: &egui::Context) {
    if self.abort_changes_in_progress() { return; }
    if let Some(op) = self.undo_stack.pop_back() {
      let inv_op = self.op_apply(ctx, op);
      self.redo_stack.push_back(inv_op);
    }
  }

  fn redo(&mut self, ctx: &egui::Context) {
    self.abort_changes_in_progress();   // Continue redo if possible
    if let Some(op) = self.redo_stack.pop_back() {
      let inv_op = self.op_apply(ctx, op);
      self.undo_stack.push_back(inv_op);
    }
  }
}

pub struct SafeTextureHandle {
  patches: Vec<SafeTexturePatch>,
  size: (f32, f32),
}

struct SafeTexturePatch {
  tex: egui::TextureHandle,
  x0: f32,
  y0: f32,
  x1: f32,
  y1: f32,
}

impl SafeTextureHandle {
  fn size(&self) -> (f32, f32) {
    self.size
  }

  fn draw(&self, painter: &egui::Painter, corners: [egui::Pos2; 4]) {
    fn interpolate(p: [egui::Pos2; 4], x: f32, y: f32) -> egui::Pos2 {
      let p_a = p[0] + (p[1] - p[0]) * x;
      let p_b = p[3] + (p[2] - p[3]) * x;
      p_a + (p_b - p_a) * y
    }
    for patch in &self.patches {
      painter.add(textured_quad([
        interpolate(corners, patch.x0, patch.y0),
        interpolate(corners, patch.x1, patch.y0),
        interpolate(corners, patch.x1, patch.y1),
        interpolate(corners, patch.x0, patch.y1),
      ], patch.tex.id()));
    }
  }
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

fn load_texture(ctx: &egui::Context, buf: crate::render::RgbaBuffer) -> SafeTextureHandle {
  let (w, h) = (buf.width as usize, buf.height as usize);

  let max_patch_size = 4096usize;
  let cols = (w + max_patch_size - 1) / max_patch_size;
  let rows = (h + max_patch_size - 1) / max_patch_size;
  let mut patches = Vec::with_capacity(cols * rows);

  for row in 0..rows {
    for col in 0..cols {
      let x0 = col * max_patch_size;
      let y0 = row * max_patch_size;
      let x1 = (x0 + max_patch_size).min(w);
      let y1 = (y0 + max_patch_size).min(h);
      let patch_w = x1 - x0;
      let patch_h = y1 - y0;

      // Extract pixels on the patch
      let mut patch_pix = Vec::with_capacity(patch_w * patch_h * 4);
      for y in y0..y1 {
        let start = (y * w + x0) * 4;
        let end = start + patch_w * 4;
        patch_pix.extend_from_slice(&buf.pixels[start..end]);
      }

      let image = egui::ColorImage::from_rgba_premultiplied(
        [patch_w, patch_h],
        &patch_pix,
      );
      let tex = ctx.load_texture("", image, egui::TextureOptions::default());
      patches.push(SafeTexturePatch {
        tex,
        x0: x0 as f32 / w as f32,
        y0: y0 as f32 / h as f32,
        x1: x1 as f32 / w as f32,
        y1: y1 as f32 / h as f32,
      });
    }
  }

  SafeTextureHandle {
    patches,
    size: (w as f32, h as f32),
  }
}

const RENDER_TEX_MARGIN: f32 = 32.0;
const RENDER_TEX_SCALE: f32 = 2.0;

fn element_to_image<P: AsRef<std::path::Path>>(root_path: P, el: &model::Element) -> crate::render::RgbaBuffer {
  use crate::{ProcessError, escape};
  fn try_to_image<P: AsRef<std::path::Path>>(root_path: P, src: &str) -> Result<crate::render::RgbaBuffer, ProcessError> {
    let src = include_str!("support.typ").to_string() +
  &format!(r#"
#set page(width: auto, height: auto, fill: none, margin: {}pt)
  "#, RENDER_TEX_MARGIN) + src;
    let buf = render::to_rgba(root_path, &src, RENDER_TEX_SCALE)
      .map_err(|e| {
        let crate::render::RenderError::TypstCompilation(diags) = e;
        ProcessError::Render(crate::RenderErrorLocation { element: Some(0), asset: diags[0].0.asset.clone() }, diags[0].1.clone())
      })?;
    Ok(buf)
  }
  match try_to_image(
    &root_path,
    &el.typst_source(&[], crate::SourceMode::Preview).unwrap(),
  ) {
    Ok(buf) => buf,
    Err(e) => {
      // XXX: Display better error messages
      try_to_image(root_path, &{
        let s = if let model::Content::Image { src, .. } = &el.content {
          format!("🖼️? {}", escape(&src))
        } else {
          // Should not happen normally; maybe fonts not loaded when migrating between machines?
          escape(&e.message())
        };
        format!("#show raw: set text(16pt, font: (\"_1\", \"_2\", \"_3\"), stroke: 2pt + white)\n#raw(\"{s}\")\n#show raw: set text(stroke: none)\n#place(top + start)[#raw(\"{s}\")]")
      }).unwrap()
    },
  }
}

fn icon_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
  ui.scope(|ui| {
    ui.spacing_mut().button_padding.x = 2.0;
    let size = ui.style().spacing.interact_size.y;
    ui.add_sized([size, size], egui::Button::selectable(false, text))
  }).inner
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
    fonts.font_data.insert("f4".to_string(),
      std::sync::Arc::new(FontData::from_static(include_bytes!("../res/NotoSansMath_Regular_subset.ttf"))));
    fonts.font_data.insert("f5".to_string(),
      std::sync::Arc::new(FontData::from_static(include_bytes!("../res/AdwaitaSans_Regular_subset.ttf"))));
    fonts.font_data.insert("f9".to_string(),
      std::sync::Arc::new(FontData::from_static(include_bytes!("../res/DroidSansFallbackFull.ttf"))));
    fonts.families.get_mut(&FontFamily::Proportional).unwrap().push("f1".to_owned());
    fonts.families.get_mut(&FontFamily::Proportional).unwrap().push("f2".to_owned());
    fonts.families.get_mut(&FontFamily::Proportional).unwrap().push("f3".to_owned());
    fonts.families.get_mut(&FontFamily::Proportional).unwrap().push("f4".to_owned());
    fonts.families.get_mut(&FontFamily::Proportional).unwrap().push("f5".to_owned());
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

    let (font_families, font_families_tag) = crate::render::font_families(None::<&str>);

    let mut self_ = Self {
      root_path: None,
      attr_panel_expanded: true,
      status_msg: None,
      yml_file_name: None,
      layout: None,
      font_families,
      font_families_tag,
      csv_file_name: None,
      csv_fields: vec![],
      asset_files: vec![],
      just_loaded: false,
      saved_at_undo: 0,
      sel_element: None,
      sel_element_scroll: false,
      dragged_element: None,
      element_texs: vec![],
      canvas: CanvasState::default(),
      close_confirming: false,
      quit_after_close: false,
      text_sel_range: None,
      attrs_panel_h: 0.6,
      attrs_panel_drag_offs: 0.,
      watcher: None,
      undo_stack: Default::default(),
      redo_stack: Default::default(),
      undo_start_state: Default::default(),
      showing_about: false,
      elements_uid: vec![],
      elements_seq: 0,
    };

    if let Some(root_path) = root_path {
      self_.load_document(root_path, ctx);
    }

    self_
  }

  fn load_document<P: AsRef<std::path::Path>>(&mut self, root_path: P, ctx: &egui::Context) {
    let root_path = root_path.as_ref();
    match self.reload_metadata(root_path, true) {
      Ok(layout) => {
        self.root_path = Some(root_path.to_path_buf());
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(
          format!("{} — Whirling Press", root_path.display())));

        self.just_loaded = true;
        self.saved_at_undo = 0;
        self.element_texs = layout.elements.iter().map(|el| {
          load_texture(ctx, element_to_image(root_path, &el))
        }).collect();

        (self.font_families, self.font_families_tag) =
          crate::render::font_families(Some(root_path));

        self.elements_uid.clear();
        for i in 0..layout.elements.len() {
          self.elements_uid.push(i);
        }
        self.elements_seq = layout.elements.len();

        self.layout = Some(layout);
        self.watcher = dir_watcher::DirWatcher::new(root_path).ok();

        self.status_msg = None;
      }
      Err(e) => {
        self.status_msg = Some((true, std::time::Instant::now(), e.message().replace(&['\r', '\n'], " ")));
      }
    }
  }

  fn reload_metadata(&mut self, root_path: &std::path::Path, parse_layout: bool) -> Result<model::Layout, crate::ProcessError> {
    match crate::process_pack_metadata(root_path, true, parse_layout) {
      Ok((yml_file_name, layout, csv_file_name, fields)) => {
        self.yml_file_name = Some(yml_file_name);
        self.csv_file_name = Some(csv_file_name);
        self.csv_fields = fields;

        // Traverse to refresh files list
        let exts = ["png", "jpg", "jpeg", "gif", "svg", "pdf", "webp"];
        self.asset_files = walkdir::WalkDir::new(&root_path).follow_links(true)
          .into_iter().filter_map(Result::ok)
          .filter(|e| {
            e.file_type().is_file() &&
            e.path().extension()
              .map_or(false, |file_ext| exts.iter().any(|target_ext| file_ext == *target_ext))
          })
          .map(|e| e.path().strip_prefix(&root_path).unwrap().to_string_lossy().into_owned())
          .collect();
        self.asset_files.sort();

        Ok(layout)
      },
      Err(e) => Err(e),
    }
  }

  fn abort_changes_in_progress(&mut self) -> bool {
    let mut changed = false;
    if let Some((i, p0, _)) = self.dragged_element.take() &&
      let Some(layout) = self.layout.as_mut()
    {
      (layout.elements[i].x, layout.elements[i].y) = p0;
      changed = true;
    }
    for (i, el) in self.undo_start_state.elements.drain() {
      if let Some(layout) = self.layout.as_mut() {
        layout.elements[i] = el;
        changed = true;
      }
    }
    changed
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
        ui.spacing_mut().button_padding = (12., 6.).into();
        if ui.button(f!("open-content-folder")).clicked() ||
          ui.input_mut(|i| i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::O)))
        {
          self.status_msg = None;
          if let Some(path) = rfd::FileDialog::new().pick_folder() {
            self.load_document(&path, ui.ctx());
          }
        }
        if false {
          ui.add_space(6.0);
          ui.label(f!("or-drag-and-drop"));
        }
      });
    });
  }

  fn ui_edit(&mut self, ui: &mut egui::Ui) {
    if let Some(watcher) = self.watcher.as_mut() &&
      let (flag_others, flag_yml) = watcher.take_changed() && (flag_others || flag_yml)
    {
      let root_path = self.root_path.as_ref().unwrap().to_path_buf();
      render::invalidate_cached_font_store(&root_path);
      match self.reload_metadata(&root_path, false) {
        Ok(_layout) => {
          if flag_others {
            // Process changed asset files
            let old_tag = self.font_families_tag;
            (self.font_families, self.font_families_tag) =
              crate::render::font_families(Some(&root_path));
            if self.font_families_tag != old_tag {
              // Fonts changed; re-render text elements
              for (el, tex) in self.layout.as_ref().unwrap().elements.iter().zip(self.element_texs.iter_mut()) {
                if let model::Content::Text { .. } = &el.content {
                  *tex = load_texture(ui.ctx(), element_to_image(&root_path, el));
                }
              }
            }
            // Unconditionally re-render image elements
            for (el, tex) in self.layout.as_ref().unwrap().elements.iter().zip(self.element_texs.iter_mut()) {
              if let model::Content::Image { .. } = &el.content {
                *tex = load_texture(ui.ctx(), element_to_image(&root_path, el));
              }
            }
          }
        }
        Err(e) => {
          self.status_msg = Some((true, std::time::Instant::now(), e.message().replace(&['\r', '\n'], " ")));
        }
      }
    }

    if !ui.ctx().egui_wants_keyboard_input() && ui.input_mut(|i|
      i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::SHIFT | egui::Modifiers::CTRL, egui::Key::Z)) ||
      i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Y))
    ) {
      self.redo(ui.ctx());
    }
    if !ui.ctx().egui_wants_keyboard_input() && ui.input_mut(|i| i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Z))) {
      self.undo(ui.ctx());
    }

    if !ui.ctx().egui_wants_keyboard_input() && ui.input_mut(|i|
      i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::Delete)) ||
      i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::Backspace))
    ) {
      if let Some(i) = self.sel_element {
        self.op(ui.ctx(), Operation::RemoveElement { i: i });
      }
    }

    let mut ops = vec![];

    use egui::{Layout, Align, Grid};

    let mut sel_element_pan = false;

    let mut attr_panel_expanded = self.attr_panel_expanded;
    egui::Panel::right("attr_panel")
      .resizable(true).default_size(320.0).size_range(80.0..=800.0)
      .show_collapsible(ui, &mut attr_panel_expanded, |ui| {
        let h = ui.available_height();
        let attrs_panel_h = if h < 240. {
          h * 0.5
        } else {
          (self.attrs_panel_h * h).clamp(120., h - 120.)
        };
        let id = self.sel_element.map_or(0_usize.wrapping_sub(1), |i| self.elements_uid[i]);
        egui::ScrollArea::vertical().id_salt(id).max_height(attrs_panel_h - 6.).show(ui, |ui| {
          ui.with_layout(Layout::top_down_justified(Align::Min), |ui| {
            if let Some(i) = self.sel_element {
              let el = &mut self.layout.as_mut().unwrap().elements[i];
              ui.heading(match &el.content {
                model::Content::Image { .. } => f!("attr-heading-image"),
                model::Content::Text(_) => f!("attr-heading-text"),
              });
              ui.push_id(
                self.elements_uid[i],
                |ui| {
                  Grid::new(("attr_grid", self.elements_uid[i]))
                    .num_columns(2)
                    .spacing([20.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                      let el = &mut self.layout.as_mut().unwrap().elements[i];
                      let el_start = el.clone();
                      let (changed_any, changed_tex, change_ended) = Self::show_attrs(
                        ui, &self.font_families, &self.csv_fields, &self.asset_files,
                        el, self.elements_uid[i], self.canvas.zoom, &mut self.text_sel_range,
                      );
                      self.undo_start_state.changed_tex |= changed_tex;
                      if changed_tex {
                        self.element_texs[i] =
                          load_texture(ui.ctx(), element_to_image(self.root_path.as_ref().unwrap(), el));
                      }
                      if changed_any {
                        self.undo_start_state.elements.entry(i).or_insert(el_start);
                      }
                      if change_ended && !ui.ctx().text_edit_focused()
                        && let Some(el_start) = self.undo_start_state.elements.remove(&i)
                      {
                        let el_after_change = std::mem::replace(el, el_start);
                        let retex = std::mem::replace(&mut self.undo_start_state.changed_tex, false);
                        self.op(ui.ctx(), Operation::ChangeElement {
                          i: i, element: el_after_change, retex: retex,
                        });
                      }
                    });
                });
            } else {
              ui.label("");
            }
          });
        });
        ui.add_space(attrs_panel_h - ui.cursor().top());
        // Draggable separator
        // Ref: egui_extras-0.36.2 src/egui_extras/table.rs
        let margin = ui.spacing().item_spacing.y;
        let (x, y) = (ui.cursor().left(), ui.cursor().top());
        let w = ui.available_width();
        let p0 = (x, y).into();
        let p1 = (x + w, y).into();
        let line_rect = egui::Rect::from_min_max(p0, p1).expand(ui.style().interaction.resize_grab_radius_side);
        let resize_response = ui.interact(line_rect, egui::Id::unique("panel_resize"), egui::Sense::click_and_drag());
        let resize_hover = resize_response.hovered() &&
          !ui.input(|i| i.pointer.any_down() || i.pointer.any_pressed());
        if resize_hover || resize_response.dragged() {
          ui.set_cursor_icon(egui::CursorIcon::ResizeRow);
        }
        if resize_response.drag_started() {
          self.attrs_panel_drag_offs = attrs_panel_h
            - ui.input(|i| i.pointer.interact_pos().unwrap().y);
        }
        if resize_response.dragged() {
          let target_h = self.attrs_panel_drag_offs
            + ui.input(|i| i.pointer.interact_pos().unwrap().y);
          self.attrs_panel_h = target_h / h;
        }
        let stroke = if resize_response.dragged() {
          ui.style().visuals.widgets.active.bg_stroke
        } else if resize_hover {
          ui.style().visuals.widgets.hovered.bg_stroke
        } else {
          ui.visuals().widgets.noninteractive.bg_stroke
        };
        ui.painter().line_segment([p0, p1], stroke);
        ui.add_space(margin * 2.);
        // Element list
        let h = ui.style().spacing.interact_size.y;
        ui.allocate_ui(ui.available_size(), |ui| {
          egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading(f!("elements"));
            // Button for adding new
            let add_resp = ui.add_sized(
              [ui.available_width(), ui.style().spacing.interact_size.y],
              egui::Button::selectable(false, "+")
            ).on_hover_text(f!("list-add"));
            egui::Popup::menu(&add_resp).id(ui.scope_id().with("add_element_popup"))
              .show(|ui| {
                ui.set_min_width(add_resp.rect.width() -
                  ui.style().spacing.menu_margin.left as f32 -
                  ui.style().spacing.menu_margin.right as f32);
                fn new_element(page: (f32, f32), origin: (f32, f32), content: model::Content) -> model::Element {
                  model::Element {
                    x: origin.0 + page.0 * 0.5,
                    y: origin.1 + page.1 * 0.5,
                    anchor: Default::default(),
                    scale: model::default_scale(),
                    rotation: Default::default(),
                    skew: Default::default(),
                    width: Default::default(),
                    height: Default::default(),
                    content,
                  }
                }
                if ui.add(egui::Button::selectable(false, format!("🖼️ {}", f!("attr-heading-image")))).clicked() {
                  let layout = &mut self.layout.as_mut().unwrap();
                  let n = layout.elements.len();
                  let page_size = layout.page;
                  self.op(ui.ctx(), Operation::AddElement { i: n, element:
                    new_element(page_size, self.canvas.origin, model::Content::Image {
                      src: "".to_string(),
                      fit: Default::default(),
                    })
                  });
                  self.sel_element = Some(n);
                }
                if ui.add(egui::Button::selectable(false, format!("💬 {}", f!("attr-heading-text")))).clicked() {
                  let layout = &mut self.layout.as_mut().unwrap();
                  let n = layout.elements.len();
                  let page_size = layout.page;
                  self.op(ui.ctx(), Operation::AddElement { i: n, element:
                    new_element(page_size, self.canvas.origin, model::Content::Text(vec![model::TextSegment {
                      family: vec![self.font_families[0].clone()],
                      size: 16.0,
                      weight: model::Weight::Regular,
                      italic: false,
                      tracking: 0.0,
                      tint: (0., 0., 0., 1.).into(),
                      text: "".to_string(),
                    }]))
                  });
                  self.sel_element = Some(n);
                }
              });
            // List
            let mut to_prioritize = None;
            let mut to_remove = None;
            let n = self.layout.as_mut().unwrap().elements.len();
            for (i, el) in self.layout.as_ref().unwrap().elements.iter().enumerate().rev() {
              let sel = self.sel_element.map_or(false, |j| j == i);
              ui.horizontal(|ui| {
                // Use the scroll area's actual content width: if the row overflows,
                // the panel's persisted outer width grows on every frame.
                let w = ui.available_width();
                let resp = ui.add_sized(
                  egui::vec2(w - (h + ui.style().spacing.item_spacing.x) * 2.0, ui.style().spacing.interact_size.y),
                  egui::Button::selectable(sel, "").left_text(
                    match &el.content {
                      model::Content::Image { src, .. } => format!("🖼️ {}", src),
                      model::Content::Text(segments) => format!("💬 {}",
                        segments.iter().map(|s| s.text.clone()).collect::<Vec<_>>().join("")),
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
                  sel_element_pan = true;
                }
                if i + 1 == n {
                  ui.add_space(h + ui.style().spacing.item_spacing.x);
                } else {
                  if icon_button(ui, "⌃").on_hover_text(f!("list-move-up")).clicked() {
                    to_prioritize = Some(i);
                  }
                }
                if icon_button(ui, "×").on_hover_text(f!("list-remove")).clicked() {
                  to_remove = Some(i);
                }
              });
            }
            if let Some(i) = to_prioritize {
              self.op(ui.ctx(), Operation::SwapElements { i: i, j: i + 1 });
            }
            if let Some(i) = to_remove {
              self.op(ui.ctx(), Operation::RemoveElement { i: i });
            }
          });
        });
      });
    self.attr_panel_expanded = attr_panel_expanded;

    for op in ops.drain(..) {
      self.op(ui.ctx(), op);
    }

    // Both central and canvas
    let (c_width, c_height) = ui.available_size().into();

    if sel_element_pan {
      // Pan canvas to element
      let page_size = &self.layout.as_ref().unwrap().page;
      let el = &self.layout.as_ref().unwrap().elements[self.sel_element.unwrap()];
      if el.x - page_size.0 * 0.5 < self.canvas.origin.0 - c_width * 0.5 / self.canvas.zoom
        || el.x - page_size.0 * 0.5 > self.canvas.origin.0 + c_width * 0.5 / self.canvas.zoom
        || el.y - page_size.1 * 0.5 < self.canvas.origin.1 - c_height * 0.5 / self.canvas.zoom
        || el.y - page_size.1 * 0.5 > self.canvas.origin.1 + c_height * 0.5 / self.canvas.zoom
      {
        self.canvas.origin = (
          el.x - page_size.0 * 0.5,
          el.y - page_size.1 * 0.5,
        );
      }
    }

    let (pos_move_precision, pos_round_precision) =
      match (100. / self.canvas.zoom).round_ties_even().clamp(1., 100.) {
        t if t < 6. => (t, 1.),
        _ => (10., 10.),
      /*
        t if t < 20. => (10., 10.),
        t if t < 60. => (10. * (t / 10.).floor(), 10.),
        _ => (100., 100.),
      */
      };
    let (pos_move_precision, pos_round_precision) =
      (pos_move_precision * 0.01, pos_round_precision * 0.01);

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
          if let Some((i, p0, delta)) = self.dragged_element {
            let layout = self.layout.as_mut().unwrap();
            self.undo_start_state.elements.entry(i).or_insert(layout.elements[i].clone());
            let new_delta = (
              delta.0 + resp.drag_delta().x / self.canvas.zoom,
              delta.1 + resp.drag_delta().y / self.canvas.zoom,
            );
            self.dragged_element = Some((i, p0, new_delta));
            let (keep_x, keep_y) = if ui.ctx().input(|i| i.modifiers.shift) {
              (delta.0.abs() < delta.1.abs(), delta.1.abs() <= delta.0.abs())
            } else {
              (false, false)
            };
            let round = |x: f32, p: f32| (x / p).round_ties_even() * p;
            (layout.elements[i].x, layout.elements[i].y) = (
              if keep_x { p0.0 } else { round(p0.0 + round(new_delta.0, pos_move_precision), pos_round_precision) },
              if keep_y { p0.1 } else { round(p0.1 + round(new_delta.1, pos_move_precision), pos_round_precision) },
            );
          } else {
            self.canvas.origin = (
              self.canvas.origin.0 - resp.drag_delta().x / self.canvas.zoom,
              self.canvas.origin.1 - resp.drag_delta().y / self.canvas.zoom,
            );
          }
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

        painter.add(egui::Shape::rect_stroke(
          egui::Rect::from_min_size(
            doc_to_canvas((0., 0.)).into(),
            [page_w * self.canvas.zoom, page_h * self.canvas.zoom].into()),
          egui::CornerRadius::same(5),
          egui::Stroke::new(1., egui::Color32::from_rgba_premultiplied(64, 64, 64, 128)),
          egui::StrokeKind::Outside,
        ));
        painter.add(egui::Shape::rect_filled(
          egui::Rect::from_min_size(
            doc_to_canvas((0., 0.)).into(),
            [page_w * self.canvas.zoom, page_h * self.canvas.zoom].into()),
          egui::CornerRadius::same(5),
          egui::Color32::from_rgba_premultiplied(255, 255, 255, 255),
        ));

        // Transforms a point in a rectangular region (texture)
        // with a pinned anchor into world coordinates
        fn transformed_anchored(
          (x0, y0): (f32, f32),   // Anchor is pinned here
          (ax, ay): (f32, f32),   // Anchor inside the texture (in original units)
          (sx, sy): (f32, f32),   // Scale
          (kx, ky): (f32, f32),   // Skew (radians)
          rota: f32,              // Rotation (radians)
          (tx, ty): (f32, f32),   // Point in question inside texture (in original units)
        ) -> (f32, f32) {
          let dx = tx - ax;
          let dy = ty - ay;
          let dx = dx * sx;
          let dy = dy * sy;
          let (dx, dy) =
            (dx + dy * kx.tan(),
             dx * ky.tan() + dy);
          let (sin, cos) = rota.sin_cos();
          let rx = dx * cos - dy * sin;
          let ry = dx * sin + dy * cos;
          (x0 + rx, y0 + ry)
        }

        fn element_tex_pos(el: &model::Element, (w, h): (f32, f32), (tx, ty): (f32, f32)) -> (f32, f32) {
          transformed_anchored(
            (el.x, el.y),
            (RENDER_TEX_MARGIN * RENDER_TEX_SCALE + (w - RENDER_TEX_MARGIN * RENDER_TEX_SCALE * 2.) * f32::from(el.anchor.x),
             RENDER_TEX_MARGIN * RENDER_TEX_SCALE + (h - RENDER_TEX_MARGIN * RENDER_TEX_SCALE * 2.) * f32::from(el.anchor.y)),
            (el.scale.0 / RENDER_TEX_SCALE, el.scale.1 / RENDER_TEX_SCALE),
            (el.skew.0.to_radians(), el.skew.1.to_radians()),
            el.rotation.to_radians(),
            (tx * w, ty * h),
          )
        }
        fn element_tex_pos_content(el: &model::Element, (w, h): (f32, f32), (tx, ty): (f32, f32)) -> (f32, f32) {
          transformed_anchored(
            (el.x, el.y),
            (RENDER_TEX_MARGIN * RENDER_TEX_SCALE + (w - RENDER_TEX_MARGIN * RENDER_TEX_SCALE * 2.) * f32::from(el.anchor.x),
             RENDER_TEX_MARGIN * RENDER_TEX_SCALE + (h - RENDER_TEX_MARGIN * RENDER_TEX_SCALE * 2.) * f32::from(el.anchor.y)),
            (el.scale.0 / RENDER_TEX_SCALE, el.scale.1 / RENDER_TEX_SCALE),
            (el.skew.0.to_radians(), el.skew.1.to_radians()),
            el.rotation.to_radians(),
            (RENDER_TEX_MARGIN * RENDER_TEX_SCALE + tx * (w - RENDER_TEX_MARGIN * RENDER_TEX_SCALE * 2.),
             RENDER_TEX_MARGIN * RENDER_TEX_SCALE + ty * (h - RENDER_TEX_MARGIN * RENDER_TEX_SCALE * 2.)),
          )
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
        let clicked_any = resp.clicked()
          || resp.clicked_by(egui::PointerButton::Secondary)
          || resp.clicked_by(egui::PointerButton::Middle);

        for (i, el) in self.layout.as_ref().unwrap().elements.iter().enumerate() {
          // XXX: DRY
          let tex = &self.element_texs[i];
          let corners_content = [
            doc_to_canvas(element_tex_pos_content(&el, tex.size(), (0., 0.))),
            doc_to_canvas(element_tex_pos_content(&el, tex.size(), (1., 0.))),
            doc_to_canvas(element_tex_pos_content(&el, tex.size(), (1., 1.))),
            doc_to_canvas(element_tex_pos_content(&el, tex.size(), (0., 1.))),
          ];
          if clicked_any {
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

        if clicked_any && first_sel.is_none() {
          self.sel_element = None;
        }

        let r = (4.0 * self.canvas.zoom).clamp(2.0, 4.0);

        for (i, el) in self.layout.as_ref().unwrap().elements.iter().enumerate() {
          let tex = &self.element_texs[i];
          let corners = [
            doc_to_canvas(element_tex_pos(&el, tex.size(), (0., 0.))).into(),
            doc_to_canvas(element_tex_pos(&el, tex.size(), (1., 0.))).into(),
            doc_to_canvas(element_tex_pos(&el, tex.size(), (1., 1.))).into(),
            doc_to_canvas(element_tex_pos(&el, tex.size(), (0., 1.))).into(),
          ];
          tex.draw(&painter, corners);

          let corners_content = [
            doc_to_canvas(element_tex_pos_content(&el, tex.size(), (0., 0.))),
            doc_to_canvas(element_tex_pos_content(&el, tex.size(), (1., 0.))),
            doc_to_canvas(element_tex_pos_content(&el, tex.size(), (1., 1.))),
            doc_to_canvas(element_tex_pos_content(&el, tex.size(), (0., 1.))),
          ];

          // Anchor display
          if self.sel_element == Some(i) {
            let anchor_canvas = doc_to_canvas((el.x, el.y));
            if self.dragged_element.is_none()
              && resp.is_pointer_button_down_on()
              && ui.input(|i| i.pointer.primary_pressed())
              && let Some(p) = ui.input(|i| i.pointer.press_origin())
              && point_in_simple_polygon(p.into(), &corners_content)
            {
              self.dragged_element = Some((i, (el.x, el.y), (0., 0.)));
            }
            painter.add(egui::Shape::Circle(egui::epaint::CircleShape {
              center: anchor_canvas.into(),
              radius: r,
              fill: if let Some((j, _, _)) = self.dragged_element && j == i {
                egui::Color32::from_rgba_premultiplied(128, 120, 96, 255)
              } else {
                egui::Color32::from_rgba_premultiplied(128, 120, 96, 128)
              },
              stroke: egui::Stroke::new(r * 0.5, egui::Color32::from_rgba_premultiplied(24, 24, 24, 255)),
            }));
          }
        }

        // Borders
        for (i, el) in self.layout.as_ref().unwrap().elements.iter().enumerate() {
          // XXX: DRY
          let tex = &self.element_texs[i];
          let corners_content = [
            doc_to_canvas(element_tex_pos_content(&el, tex.size(), (0., 0.))),
            doc_to_canvas(element_tex_pos_content(&el, tex.size(), (1., 0.))),
            doc_to_canvas(element_tex_pos_content(&el, tex.size(), (1., 1.))),
            doc_to_canvas(element_tex_pos_content(&el, tex.size(), (0., 1.))),
          ];
          let mut tint = None;
          if let Some(p) = hover_pos && point_in_simple_polygon(p, &corners_content) {
            tint = Some((
              egui::Color32::from_rgba_premultiplied(24, 24, 24, 64),
              egui::Color32::from_rgba_premultiplied(64, 64, 64, 64),
            ));
          }
          if self.sel_element == Some(i) {
            tint = Some((
              egui::Color32::from_rgba_premultiplied(24, 24, 24, 255),
              egui::Color32::from_rgba_premultiplied(255, 255, 255, 255),
            ));
          }
          if let Some((t1, t2)) = tint {
            painter.line(
              vec![
                corners_content[0].into(), corners_content[1].into(),
                corners_content[2].into(), corners_content[3].into(),
                corners_content[0].into()
              ],
              egui::Stroke::new(r * 0.5 + 1., t2),
            );
            painter.line(
              vec![
                corners_content[0].into(), corners_content[1].into(),
                corners_content[2].into(), corners_content[3].into(),
                corners_content[0].into()
              ],
              egui::Stroke::new(r * 0.5, t1),
            );
          }
        }

        if !resp.is_pointer_button_down_on() && let Some((i, _, _)) = self.dragged_element.take() {
          if !ui.ctx().text_edit_focused()
            && let Some(el_start) = self.undo_start_state.elements.remove(&i)
          {
            let el = &mut self.layout.as_mut().unwrap().elements[i];
            let el_after_change = std::mem::replace(el, el_start);
            self.op(ui.ctx(), Operation::ChangeElement {
              i: i, element: el_after_change, retex: false,
            });
          }
        }

        // Panel toggle
        egui::Area::new(ui.scope_id().with("attr_panel_toggle"))
          .fixed_pos(egui::pos2(c_width, c_height / 2.0))
          .pivot(egui::Align2::RIGHT_CENTER)
          .show(ui.ctx(), |ui| {
            if ui.add(egui::Button::new(if self.attr_panel_expanded { "▸" } else { "◂" }))
              .on_hover_text(if self.attr_panel_expanded { f!("hide-attrs") } else { f!("hide-attrs") })
              .clicked()
            {
              self.attr_panel_expanded = !self.attr_panel_expanded;
            }
          });
      });
  }

  fn show_attrs(ui: &mut egui::Ui, font_families: &[String], csv_fields: &[String], asset_files: &[String], el: &mut model::Element, el_uid: usize, zoom: f32, text_sel_range: &mut Option<(usize, usize)>) -> (bool, bool, bool) {
    let mut changed_any = false;
    let mut changed_tex = false;
    let mut change_ended = false;
    use egui::{DragValue, ComboBox};
    ui.label(f!("attr-position"));
    ui.horizontal(|ui| {
      let s = 0.1 * (10. / zoom).round_ties_even().max(1.);
      let resp_x = ui.add_sized([80.0, ui.style().spacing.interact_size.y], DragValue::new(&mut el.x).speed(s).min_decimals(1));
      let resp_y = ui.add_sized([80.0, ui.style().spacing.interact_size.y], DragValue::new(&mut el.y).speed(s).min_decimals(1));
      changed_any |= resp_x.changed() || resp_y.changed();
      change_ended |= resp_x.drag_stopped() || resp_x.lost_focus() || resp_y.drag_stopped() || resp_y.lost_focus();
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
                if matches!(el.content, model::Content::Text { .. }) {
                  changed_tex = true;
                }
                if let model::Content::Image{ fit, .. } = el.content &&
                  fit != model::ImageFit::Stretch
                {
                  changed_tex = true;
                }
                change_ended = true;
              }
            }
            ui.end_row();
          }
        }));
    ui.end_row();
    ui.label(f!("attr-scale"));
    ui.horizontal(|ui| {
      let resp_x = ui.add_sized([80.0, ui.style().spacing.interact_size.y], DragValue::new(&mut el.scale.0).speed(0.01));
      let resp_y = ui.add_sized([80.0, ui.style().spacing.interact_size.y], DragValue::new(&mut el.scale.1).speed(0.01));
      changed_any |= resp_x.changed() || resp_y.changed();
      change_ended |= resp_x.drag_stopped() || resp_x.lost_focus() || resp_y.drag_stopped() || resp_y.lost_focus();
    });
    ui.end_row();
    ui.label(f!("attr-skew"));
    ui.horizontal(|ui| {
      let resp_x = ui.add_sized([80.0, ui.style().spacing.interact_size.y], DragValue::new(&mut el.skew.0).speed(0.1).range(-89.9..=89.9));
      let resp_y = ui.add_sized([80.0, ui.style().spacing.interact_size.y], DragValue::new(&mut el.skew.1).speed(0.1).range(-89.9..=89.9));
      changed_any |= resp_x.changed() || resp_y.changed();
      change_ended |= resp_x.drag_stopped() || resp_x.lost_focus() || resp_y.drag_stopped() || resp_y.lost_focus();
    });
    ui.end_row();
    ui.label(f!("attr-rotation"));
    ui.horizontal(|ui| {
      let resp = ui.add_sized([80.0, ui.available_height()], DragValue::new(&mut el.rotation).speed(0.1));
      changed_any |= resp.changed();
      change_ended |= resp.drag_stopped() || resp.lost_focus();
    });
    ui.end_row();
    ui.label(f!("attr-size-limit"));
    fn size_limit_editor(ui: &mut egui::Ui, value: &mut Option<f32>) -> (bool, bool) {
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
      (resp.changed(), resp.lost_focus())
    }
    ui.horizontal(|ui| {
      let (a, b) = size_limit_editor(ui, &mut el.width);
      changed_tex |= a; change_ended |= b;
      let (a, b) = size_limit_editor(ui, &mut el.height);
      changed_tex |= a; change_ended |= b;
    });
    ui.end_row();
    use egui::TextBuffer;
    fn field_range(text: &str, cursor: usize) -> Option<(usize, usize)> {
      if cursor > text.chars().count() {
        return None;
      }
      let byte_cursor = text.byte_index_from_char_index(cursor.into()).into();
      let bytes = text.as_bytes();
      let mut open = None;
      for i in (0..byte_cursor).rev() {
        match bytes[i] {
          b'}' => return None,
          b'{' => {
            open = Some(i);
            break;
          }
          _ => {}
        }
      }
      let open = open?;
      let mut end = text.len();
      for j in byte_cursor..text.len() {
        match bytes[j] {
          b'}' => {
            end = j;
            break;
          }
          b'{' => return None,
          _ => {}
        }
      }
      Some((
        text.char_index_from_byte_index(open.into()).into(),
        text.char_index_from_byte_index((end + 1).into()).into(),
      ))
    }
    fn show_autocomplete(ui: &egui::Ui, edit_output: egui::widgets::text_edit::TextEditOutput, src: &mut String, text_sel_range: &mut Option<(usize, usize)>, replace_range: Option<(usize, usize)>, csv_fields: &[String], extras: &[String]) -> (bool, bool) {
      let mut edit_output = edit_output;
      if let Some((start, end)) = text_sel_range.take() {
        edit_output.state.cursor.set_char_range(Some(
          egui::text_selection::CCursorRange::two(
            egui::text::CCursor::new(start - 1),
            egui::text::CCursor::new(end + 1),
          )
        ));
        edit_output.response.request_focus();
      }
      let resp = &mut edit_output.response;
      let replace_range = replace_range.map(|(start, end)|
        egui::text_selection::CCursorRange::two(
          egui::text::CCursor::new(start),
          egui::text::CCursor::new(end),
        )
      );
      let replace_text = (if let Some(r) = replace_range { r.slice_str(src) } else { "" }).to_owned();
      egui::Popup::from_response(&resp)
        .open_memory(resp.has_focus().then_some(egui::containers::SetOpenCommand::Bool(true)))
        .kind(egui::PopupKind::Menu)
        .layout(egui::Layout::top_down_justified(egui::Align::Min))
        .style(egui::containers::menu::menu_style)
        .show(|ui| {
          ui.set_min_width(resp.rect.width() -
            ui.style().spacing.menu_margin.left as f32 -
            ui.style().spacing.menu_margin.right as f32);
          egui::ScrollArea::vertical()
            .max_height(ui.style().spacing.combo_height)
            .show(ui, |ui| {
              ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
              use std::borrow::Cow;
              let full_match =
                csv_fields.iter().map(|field| Cow::Owned(format!("{{{}}}", field)))
                  .chain(extras.into_iter().map(Cow::Borrowed))
                  .any(|option| *src == *option);
              for option in csv_fields.iter().map(|field| Cow::Owned(format!("{{{}}}", field)))
                .chain(extras.into_iter().map(Cow::Borrowed))
              {
                let replaces_all = matches!(option, Cow::Borrowed(_));
                let menu_resp = ui.add(egui::Button::selectable(*src == *option || replace_text == *option, &*option));
                if (resp.gained_focus() || resp.changed()) &&
                  src.len() > 0 && option.starts_with(&*src)
                {
                  menu_resp.scroll_to_me_animation(None,
                    egui::style::ScrollAnimation::new(1500., egui::Rangef::new(0.1, 0.15)));
                }
                if menu_resp.clicked() {
                  let cursor = src.delete_selected(&if replaces_all || full_match {
                    egui::text_selection::CCursorRange::two(
                      egui::text::CCursor::new(0),
                      egui::text::CCursor::new(src.chars().count()),
                    )
                  } else if let Some(r) = edit_output.cursor_range && !r.is_empty() &&
                    (replace_range.is_none() || !replace_range.unwrap().contains(r))
                  {
                    r
                  } else if let Some(r) = replace_range {
                    r
                  } else {
                    egui::text_selection::CCursorRange::one(
                      egui::text::CCursor::new(src.chars().count())
                    )
                  });
                  let cursor = cursor.index + src.insert_text(&option, cursor.index);
                  edit_output.state.cursor.set_char_range(Some(
                    egui::text_selection::CCursorRange::one(
                      egui::text::CCursor::new(cursor))
                  ));
                  resp.mark_changed();
                  resp.request_focus();
                }
              }
            }).inner
        });
      edit_output.state.store(ui.ctx(), edit_output.response.id);
      (edit_output.response.changed(), edit_output.response.lost_focus())
    }
    match &mut el.content {
      model::Content::Image { src, fit } => {
        ui.label(f!("attr-image-source"));
        let edit_output = egui::TextEdit::singleline(src).show(ui);
        let range = if let Some(c) = edit_output.state.cursor.char_range() {
          field_range(src, c.primary.index.into())
        } else { None };
        let (a, b) = show_autocomplete(ui, edit_output, src, text_sel_range, range, csv_fields, &asset_files);
        changed_tex |= a; change_ended |= b;
        ui.end_row();
        ui.label(f!("attr-image-fit"));
        let fits_and_names = [
          (model::ImageFit::Contain, f!("attr-image-fit-contain")),
          (model::ImageFit::Cover, f!("attr-image-fit-cover")),
          (model::ImageFit::Stretch, f!("attr-image-fit-stretch")),
        ];
        ComboBox::from_id_salt("fit")
          .width(150.0)
          .selected_text(fits_and_names.iter()
            .find(|(a, _)| *a == *fit)
            .map(|(_, name)| name.clone()).unwrap())
          .show_ui(ui, |ui| {
            for (value, name) in fits_and_names {
              if ui.selectable_value(fit, value, name).clicked() {
                changed_tex = true;
                change_ended = true;
              }
            }
          });
        ui.end_row();
      },
      model::Content::Text(segments) => {
        let n = segments.len();
        let mut to_prioritize = None;
        let mut to_remove = None;
        let base_id = ui.scope_id();
        let segment_uids_id = egui::Id::unique(("text_segment_uids", el_uid));
        let mut segment_uids = ui.ctx().memory_mut(|mem|
          mem.data.get_temp::<Vec<usize>>(segment_uids_id)).unwrap_or_default();
        if segment_uids.len() != n {
          segment_uids = (0..n).collect();
        }
        let layout_dirty_id = egui::Id::unique(("text_segment_layout_dirty", el_uid));
        if ui.ctx().memory_mut(|mem| mem.data.remove_temp::<bool>(layout_dirty_id)).unwrap_or(false) {
          ui.ctx().request_discard("text segment layout changed");
        }
        let mark_layout_dirty = |ui: &egui::Ui| {
          ui.ctx().memory_mut(|mem| mem.data.insert_temp(layout_dirty_id, true));
        };
        for (j, model::TextSegment { family, size, weight, italic, tracking, tint, text }) in segments.iter_mut().enumerate() {
          let seg_id = base_id.with(("text_segment", segment_uids[j]));
          let collapse_id = seg_id.with("collapse");
          let is_collapsed = ui.ctx().memory_mut(|mem| mem.data.get_temp::<bool>(collapse_id).unwrap_or(false));
          if n > 1 {
            let resp = ui.add(
              egui::Label::new(egui::RichText::new(format!("{} #{}", if is_collapsed { "▸" } else { "▾" }, j + 1)).weak())
                .sense(egui::Sense::HOVER | egui::Sense::CLICK | egui::Sense::FOCUSABLE));
            if resp.hovered() {
              ui.set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if resp.clicked() {
              ui.ctx().memory_mut(|mem| mem.data.insert_temp(collapse_id, !is_collapsed));
              mark_layout_dirty(ui);
            }
            ui.horizontal(|ui| {
              let h = ui.style().spacing.interact_size.y;
              let (rect, resp) = ui.allocate_exact_size(
                [ui.available_width() - (h + ui.style().spacing.item_spacing.x) *
                  if j > 0 { 2. } else { 1. }, h].into(),
                egui::Sense::HOVER | egui::Sense::CLICK | egui::Sense::FOCUSABLE);
              ui.scope_builder(
                egui::UiBuilder::new().max_rect(rect)
                  .layout(egui::Layout::top_down_justified(egui::Align::Min)),
                |ui| if is_collapsed && n > 1 {
                  ui.add(egui::Label::new(egui::RichText::new(text.clone()).weak())
                    .wrap_mode(egui::TextWrapMode::Truncate)
                    .selectable(false)
                  );
                } else {
                  ui.add_space(h / 2. - 1.); ui.separator();
                },
              );
              if n > 1 {
                if resp.hovered() {
                  ui.set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if resp.clicked() {
                  ui.ctx().memory_mut(|mem| mem.data.insert_temp(collapse_id, !is_collapsed));
                  mark_layout_dirty(ui);
                }
              }
              if j > 0 {
                if icon_button(ui, "⌃").on_hover_text(f!("list-move-up")).clicked() {
                  to_prioritize = Some(j);
                }
              }
              if icon_button(ui, "×").on_hover_text(f!("list-remove")).clicked() {
                to_remove = Some(j);
              }
            });
            ui.end_row();
          }
          if is_collapsed && n > 1 { continue; }
          ui.label(f!("attr-text-text"));
          let edit_output = egui::TextEdit::multiline(text)
            .desired_width(ui.available_width()).id(seg_id.with("text")).show(ui);
          let range = if let Some(c) = edit_output.state.cursor.char_range() {
            field_range(text, c.primary.index.into())
          } else { None };
          if range.is_some() {
            let (a, b) = show_autocomplete(ui, edit_output, text, text_sel_range, range, csv_fields, &[]);
            changed_tex |= a; change_ended |= b;
          } else {
            changed_tex |= edit_output.response.changed();
            change_ended |= edit_output.response.lost_focus();
          }
          ui.end_row();
          ui.label(f!("attr-text-family"));
          let fonts_changed = ui.scope_builder(
            egui::UiBuilder::new().scope_id(seg_id.with("family")),
            |ui| Self::font_list_editor(ui, &font_families, family)
          ).inner;
          if fonts_changed {
            mark_layout_dirty(ui);
          }
          changed_tex |= fonts_changed;
          change_ended |= fonts_changed;
          ui.end_row();
          ui.label(f!("attr-text-size"));
          let resp = ui.add_sized([80.0, ui.style().spacing.interact_size.y], DragValue::new(size).range(0.0..=f32::INFINITY).speed(1.0));
          changed_tex |= resp.changed();
          change_ended |= resp.drag_stopped() || resp.lost_focus();
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
          ComboBox::from_id_salt(seg_id.with("weight"))
            .width(150.0)
            .selected_text(weights_and_names.iter()
              .find(|(a, _)| *a == *weight)
              .map(|(_, name)| *name).unwrap())
            .show_ui(ui, |ui| {
              for (value, name) in weights_and_names {
                if ui.selectable_value(weight, value, name).clicked() {
                  changed_tex = true;
                  change_ended = true;
                }
              }
            });
          ui.end_row();
          ui.label(f!("attr-text-italic"));
          let resp = ui.scope_builder(egui::UiBuilder::new().scope_id(seg_id.with("italic")), |ui| ui.checkbox(italic, "")).inner;
          changed_tex |= resp.changed();
          change_ended |= resp.changed();
          ui.end_row();
          ui.label(f!("attr-text-tracking"));
          let resp = ui.add_sized([80.0, ui.style().spacing.interact_size.y], DragValue::new(tracking).speed(0.1));
          changed_tex |= resp.changed();
          change_ended |= resp.drag_stopped() || resp.lost_focus();
          ui.end_row();
          ui.label(f!("attr-text-tint"));
          ui.horizontal(|ui| {
            let h = ui.style().spacing.interact_size.y;
            let spacing = ui.style().spacing.item_spacing.x;
            let is_field = matches!(tint, model::FieldOrLiteral::Field(_));
            match tint {
              model::FieldOrLiteral::Field(field) => {
                let edit_output = egui::TextEdit::singleline(field)
                  .desired_width((ui.available_width() - (h + spacing)).max(80.)).show(ui);
                let range = edit_output.state.cursor.char_range()
                  .and_then(|c| field_range(field, c.primary.index.into()));
                let (a, b) = show_autocomplete(ui, edit_output, field, text_sel_range, range, csv_fields, &[]);
                changed_tex |= a; change_ended |= b;
              },
              model::FieldOrLiteral::Literal(color) => {
                let (a, b) = color_picker::color_edit_button_srgba(ui, color, color_picker::Alpha::OnlyBlend);
                fn round_dec(x: f32) -> f32 { (x * 1000.).round_ties_even() / 1000. }
                *color = (round_dec(color.0), round_dec(color.1), round_dec(color.2), round_dec(color.3));
                changed_tex |= a; change_ended |= b;
              },
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
              if icon_button(ui, "↔").on_hover_text(
                if is_field { f!("attr-text-tint-switch-to-fixed") }
                else { f!("attr-text-tint-switch-to-field") }
              ).clicked() {
                let field_id = seg_id.with("tint_field");
                let fixed_id = seg_id.with("tint_fixed");
                *tint = if is_field {
                  if let model::FieldOrLiteral::Field(field) = tint {
                    ui.ctx().memory_mut(|mem| mem.data.insert_temp(field_id, field.clone()));
                  }
                  let value_fixed = ui.ctx().memory_mut(|mem|
                    mem.data.get_temp::<(f32, f32, f32, f32)>(fixed_id)).unwrap_or((0., 0., 0., 1.));
                  model::FieldOrLiteral::Literal(value_fixed)
                } else {
                  if let model::FieldOrLiteral::Literal(tint) = tint {
                    ui.ctx().memory_mut(|mem| mem.data.insert_temp(fixed_id, *tint));
                  }
                  let field = ui.ctx().memory_mut(|mem|
                    mem.data.get_temp::<String>(field_id)).unwrap_or_default();
                  model::FieldOrLiteral::Field(field)
                };
                changed_tex = true;
                change_ended = true;
                mark_layout_dirty(ui);
              }
            });
          });
          ui.end_row();
        }
        ui.label("");
        ui.horizontal(|ui| {
          let h = ui.style().spacing.interact_size.y;
          ui.allocate_ui_with_layout(
            [ui.available_width() - (h + ui.style().spacing.item_spacing.x), h].into(),
            egui::Layout::top_down_justified(egui::Align::Center),
            |ui| { ui.add_space(h / 2. - 1.); ui.separator(); },
          );
          if icon_button(ui, "+").on_hover_text(f!("list-add-segment")).clicked() {
            segments.push(model::TextSegment {
              text: "".to_string(),
              ..segments.last().unwrap().clone()
            });
            let new_uid = segment_uids.iter().max().map_or(0, |m| m + 1);
            segment_uids.push(new_uid);
            mark_layout_dirty(ui);
            changed_any = true;
            change_ended = true;
            // No need to re-render, as the text is empty
            ui.scroll_with_delta_animation([0., -ui.style().spacing.interact_size.y * 10.].into(),
              egui::style::ScrollAnimation::new(1500., egui::Rangef::new(0.1, 0.15)));
          }
        });
        ui.end_row();
        if let Some(i) = to_prioritize {
          segments.swap(i, i - 1);
          segment_uids.swap(i, i - 1);
          mark_layout_dirty(ui);
          changed_tex = true;
          change_ended = true;
        }
        if let Some(i) = to_remove {
          segments.remove(i);
          segment_uids.remove(i);
          mark_layout_dirty(ui);
          changed_tex = true;
          change_ended = true;
        }
        ui.ctx().memory_mut(|mem| mem.data.insert_temp(segment_uids_id, segment_uids));
      },
    }
    (changed_any || changed_tex, changed_tex, change_ended)
  }

  fn font_list_editor(ui: &mut egui::Ui, font_families: &[String], el: &mut Vec<String>) -> bool {
    let mut changed = false;
    ui.vertical(|ui| {
      let mut to_prioritize = None;
      let mut to_remove = None;
      let n = el.len();
      for (i, s) in el.iter_mut().enumerate() {
        // ui.dnd_drag_source(ui.scope_id().with(("font_list", i)), i, |ui| {
        ui.horizontal(|ui| {
          let h = ui.available_height();
          let button_w = if n > 1 {
            ui.available_width() - (h + ui.style().spacing.item_spacing.x) * 2.0
          } else {
            ui.available_width()
          };
          let resp = ui.add_sized([button_w, h],
            egui::Button::selectable(false, &*s).wrap_mode(egui::TextWrapMode::Truncate));
          egui::Popup::menu(&resp).id(ui.scope_id().with(("font_family_popup", i)))
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
              if icon_button(ui, "⌃").on_hover_text(f!("list-move-up")).clicked() {
                to_prioritize = Some(i);
              }
            }
            if icon_button(ui, "×").on_hover_text(f!("list-remove")).clicked() {
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

  fn export<P: AsRef<std::path::Path>>(&mut self, path: P, format: crate::ArgsFormat) {
    let path = path.as_ref();
    match crate::process_and_write(
      self.root_path.as_ref().unwrap(),
      Some(self.layout.as_ref().unwrap()),
      path,
      format,
      true,
    ) {
      Ok(_) => self.status_msg = Some((false, std::time::Instant::now(), f!("status-exported", path => path.to_string_lossy()))),
      Err(crate::ProcessError::Expr(index, crate::ExprError::NonexistentField(start, end, name))) => {
        self.sel_element = Some(index);
        self.sel_element_scroll = true;
        self.status_msg = Some((true, std::time::Instant::now(), f!("status-nonexistent-field", name => name.replace(&['\r', '\n'], " "))));
        self.text_sel_range = Some((start, end));
      },
      Err(crate::ProcessError::Render(location, msg)) => {
        if let Some(index) = location.element {
          self.sel_element = Some(index);
          self.sel_element_scroll = true;
        }
        let msg = msg.replace(&['\r', '\n'], " ");
        self.status_msg = Some((true, std::time::Instant::now(), location.describe(&msg)));
      },
      Err(e) => self.status_msg = Some((true, std::time::Instant::now(), format!("{}", e.message()))),
    }
  }
}

impl eframe::App for WhirlApp {
  fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
    egui::Panel::bottom("status_bar")
      .frame(egui::Frame::side_top_panel(ui.style()))
      .show(ui, |ui| {
        let close_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::W);
        let mut to_close = ui.input_mut(|i| i.consume_shortcut(&close_shortcut));
        let modified = !(self.saved_at_undo == self.undo_stack.len());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
          // `egui::widgets::global_theme_preference_switch` uses a frameless button
          // which has less-than-ideal hover feedback
          let is_dark = ui.ctx().theme() == egui::Theme::Dark;
          if ui.add_sized([ui.available_height(), ui.available_height()],
            egui::Button::selectable(false,
              egui::RichText::new(if is_dark { "🌙" } else { "☀️" }).size(14.0)))
            .on_hover_text(f!("status-button-theme")).clicked()
          {
            ui.ctx().set_theme(if is_dark { egui::Theme::Light } else { egui::Theme::Dark });
          }
          if ui.add_sized([ui.available_height(), ui.available_height()],
            egui::Button::selectable(false,
              egui::RichText::new("🌐").size(14.0)))
            .on_hover_text(f!("status-button-language")).clicked()
          {
            localize::set_language(match localize::current_language().to_string() {
              v if v == "en" => "zh-Hans",
              v if v == "zh-Hans" => "en",
              _ => "en",
            });
          }
          ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            if let Some(path) = &self.root_path {
              let path_str = path.to_string_lossy().into_owned();
              // Global shortcuts
              let save_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::S);
              let mut to_save = ui.input_mut(|i| i.consume_shortcut(&save_shortcut));
              let export_p_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::P);
              let mut to_export_p = ui.input_mut(|i| i.consume_shortcut(&export_p_shortcut));
              // Widgets
              let resp = ui.add_sized([ui.available_height(), ui.available_height()],
                egui::Button::new(egui::RichText::new("📄").size(14.0)))
                .on_hover_text(f!("status-button-menu"));
              let popup_id = resp.id.with("popup");
              egui::Popup::menu(&resp)
              .id(popup_id)
              .close_behavior(egui::containers::PopupCloseBehavior::CloseOnClickOutside)
              .show(|ui| {
                ui.horizontal(|ui| {
                  ui.label(f!("menu-page-size"));
                  let page_size_start = self.layout.as_ref().unwrap().page;
                  let resp_x = ui.add_sized([80.0, ui.style().spacing.interact_size.y], egui::DragValue::new(&mut self.layout.as_mut().unwrap().page.0).speed(1).range(1.0..=f32::INFINITY));
                  let resp_y = ui.add_sized([80.0, ui.style().spacing.interact_size.y], egui::DragValue::new(&mut self.layout.as_mut().unwrap().page.1).speed(1).range(1.0..=f32::INFINITY));
                  if (resp_x.drag_stopped() || resp_x.lost_focus() ||
                      resp_y.drag_stopped() || resp_y.lost_focus())
                    && !ui.ctx().text_edit_focused()
                    && let Some(p) = self.undo_start_state.page_size.take()
                    && p != self.layout.as_ref().unwrap().page
                  {
                    let page_size = self.layout.as_ref().unwrap().page;
                    self.layout.as_mut().unwrap().page = p; // For proper undo
                    self.op(ui.ctx(), Operation::ChangePageSize { page_size: page_size });
                  }
                  if resp_x.drag_started() || resp_x.gained_focus() {
                    self.undo_start_state.page_size = Some(page_size_start);
                  }
                  if resp_y.drag_started() || resp_y.gained_focus() {
                    self.undo_start_state.page_size = Some(page_size_start);
                  }
                });
                ui.separator();
                ui.spacing_mut().button_padding = (0., 3.).into();
                if ui.add(egui::Button::new(format!("💾 {}", f!("menu-save"))).shortcut_text(ui.ctx().format_shortcut(&save_shortcut))).clicked() {
                  // Save YAML
                  to_save = true;
                  egui::Popup::close_id(ui.ctx(), popup_id);
                }
                if ui.add_enabled(
                  self.undo_stack.len() > 0,
                  egui::Button::new(format!("↶ {}", f!("menu-undo"))).shortcut_text(ui.ctx().format_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Z))),
                ).clicked() {
                  self.undo(ui.ctx());
                  egui::Popup::close_id(ui.ctx(), popup_id);
                }
                if ui.add_enabled(
                  self.redo_stack.len() > 0,
                  egui::Button::new(format!("↷ {}", f!("menu-redo"))).shortcut_text(ui.ctx().format_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Y))),
                ).clicked() {
                  self.redo(ui.ctx());
                  egui::Popup::close_id(ui.ctx(), popup_id);
                }
                ui.separator();
                if ui.add(egui::Button::new(format!("📂 {}", f!("menu-show-content-folder")))).clicked() {
                  if let Err(e) = opener::open(&self.root_path.as_ref().unwrap()) {
                    self.status_msg = Some((true, std::time::Instant::now(), f!("status-open-error", message => format!("{}", e))));
                  }
                }
                if ui.add(egui::Button::new(format!("📂 {}", f!("menu-show-spreadsheet")))).clicked() {
                  if let Err(e) = opener::reveal(&self.root_path.as_ref().unwrap().join(self.csv_file_name.as_ref().unwrap())) {
                    self.status_msg = Some((true, std::time::Instant::now(), f!("status-open-error", message => format!("{}", e))));
                  }
                }
                ui.separator();
                if ui.add(egui::Button::new(format!("🖨 {}", f!("menu-export-pdf"))).shortcut_text(ui.ctx().format_shortcut(&export_p_shortcut))).clicked() {
                  // Export PDF
                  to_export_p = true;
                  egui::Popup::close_id(ui.ctx(), popup_id);
                }
                ui.menu_button(format!("🖨 {}", f!("menu-export-other")), |ui| {
                  ui.spacing_mut().button_padding = (2., 3.).into();
                  if ui.add(egui::Button::new(f!("menu-export-pdf-seq"))).clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                      self.export(&path, crate::ArgsFormat::PdfSeq);
                    }
                    egui::Popup::close_id(ui.ctx(), popup_id);
                  }
                  if ui.add(egui::Button::new(f!("menu-export-png-seq"))).clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                      self.export(&path, crate::ArgsFormat::PngSeq);
                    }
                    egui::Popup::close_id(ui.ctx(), popup_id);
                  }
                  if ui.add(egui::Button::new(f!("menu-export-typst"))).clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("Typst", &["typ"]).save_file() {
                      self.export(&path, crate::ArgsFormat::Typst);
                    }
                    egui::Popup::close_id(ui.ctx(), popup_id);
                  }
                });
                ui.separator();
                if ui.add(egui::Button::new(format!("⬅️ {}", f!("menu-close"))).shortcut_text(ui.ctx().format_shortcut(&close_shortcut))).clicked() {
                  to_close = true;
                  egui::Popup::close_id(ui.ctx(), popup_id);
                }
              });
              // Shortcut operations
              if to_save {
                let yml_file_name = self.yml_file_name.as_ref().unwrap();
                let yml_path = self.root_path.as_ref().unwrap().join(yml_file_name);
                let s = serde_saphyr::to_string_with_options(self.layout.as_ref().unwrap(), serde_saphyr::ser_options! {
                  compact_list_indent: true,
                  yaml_12: true,
                }).unwrap();
                match std::fs::write(&yml_path, s) {
                  Ok(_) => self.status_msg = Some((false, std::time::Instant::now(), f!("status-saved", path => yml_file_name.to_string_lossy()))),
                  Err(e) => self.status_msg = Some((true, std::time::Instant::now(), format!("{}", e))),
                }
                self.saved_at_undo = self.undo_stack.len();
              }
              if to_export_p {
                if let Some(path) = rfd::FileDialog::new()
                  .add_filter("PDF", &["pdf"])
                  .save_file()
                {
                  self.export(&path, crate::ArgsFormat::Pdf);
                }
              }
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
                let layout = self.layout.as_ref().unwrap();
                ui.label(format!("{}{}×{}, {} — {}",
                  if modified { "* " } else { "" },
                  layout.page.0, layout.page.1,
                  f!("status-num-elements", n => layout.elements.len()),
                  path_str));
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
            } else {
              let resp = ui.add(egui::Label::new(
                egui::widget_text::WidgetText::from(crate::version()).color(ui.style().visuals.weak_text_color()))
                .selectable(false)
                .sense(egui::Sense::HOVER | egui::Sense::CLICK | egui::Sense::FOCUSABLE)
              ).on_hover_text(f!("status-button-about"));
              if resp.hovered() {
                ui.set_cursor_icon(egui::CursorIcon::PointingHand);
              }
              if resp.clicked() {
                self.showing_about = true;
              }
              if self.showing_about {
                let modal_resp = egui::Modal::new(egui::Id::unique(format!("about_modal_{}", localize::current_language()))).show(ui.ctx(), |ui| {
                  ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
                    ui.spacing_mut().button_padding = (12., 6.).into();
                    ui.heading("Whirling Press");
                    ui.label(format!("{}　{}", crate::version(), crate::commit_date()));
                    ui.separator();
                    ui.add_space(12.0);
                    ui.label(f!("modal-about-description"));
                    ui.add_space(12.0);
                    if ui.hyperlink_to("ayu.land/whirl", "https://ayu.land/whirl")
                      .on_hover_text(f!("modal-about-homepage")).clicked()
                    {
                      let _ = opener::open("https://ayu.land/whirl");
                    }
                    ui.add_space(12.0);
                    if ui.button(f!("modal-about-close")).clicked() ||
                      ui.input_mut(|i| i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::Enter)))
                    {
                      self.showing_about = false;
                    }
                  });
                });
                if modal_resp.should_close() {
                  self.showing_about = false;
                }
              }
            }
          });
        });
        let mut really_close = false;
        if to_close {
          if modified {
            self.close_confirming = true;
          } else {
            really_close = true;
          }
        }
        // `quit_after_close` is also used as a flag to denote confirmed quit
        // so that the close is not intercepted by `CancelClose`
        if modified && ui.input(|i| i.viewport().close_requested()) && !self.quit_after_close {
          self.close_confirming = true;
          self.quit_after_close = true;
          ui.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        if self.close_confirming {
          egui::Modal::new(egui::Id::unique("close_confirmation_modal")).show(ui.ctx(), |ui| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
              ui.label(f!("modal-close-confirmation-message"));
              ui.add_space(12.0);
              ui.spacing_mut().button_padding = (12., 6.).into();
              if ui.button(f!("modal-close-confirmation-close")).clicked() ||
                ui.input_mut(|i| i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::Enter)))
              {
                self.close_confirming = false;
                really_close = true;
              }
              ui.add_space(6.0);
              if ui.button(f!("modal-close-confirmation-cancel")).clicked() ||
                ui.input_mut(|i| i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::Escape)))
              {
                self.close_confirming = false;
                self.quit_after_close = false;
              }
            });
          });
        }
        if really_close {
          if self.quit_after_close {
            ui.send_viewport_cmd(egui::ViewportCommand::Close);
          } else {
            self.root_path = None;
            self.status_msg = None;
            self.yml_file_name = None;
            self.csv_file_name = None;
            self.layout = None;
            self.font_families.clear();
            self.element_texs.clear();
            self.watcher = None;
            self.undo_stack.clear();
            self.redo_stack.clear();
            self.undo_start_state = Default::default();
            self.elements_uid.clear();
            self.elements_seq = 0;
          }
        }
      });
    if let Some(_) = &self.root_path {
      self.ui_edit(ui);
    } else {
      self.ui_landing(ui);
    }
    if let Some((_, t, _)) = self.status_msg
      && t.elapsed() > std::time::Duration::from_secs(5)
    {
      self.status_msg = None;
    }
  }
}
