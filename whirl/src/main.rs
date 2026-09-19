#![windows_subsystem = "windows"]

mod model;
mod render;
mod opt_pdf;
mod localize;
#[cfg(feature = "gui")]
mod gui;

#[derive(Debug)]
enum ExprError {
  NonexistentField(usize, String),
}

fn escape(s: &str) -> String {
  s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn expand_expr(s: &str, fields: &[String]) -> Result<String, ExprError> {
  let mut result = Vec::new();
  let mut cur = 0;

  fn find_next_char(s: &str, t: char, byte_offs: usize) -> Option<usize> {
    s.get(byte_offs..)?.find(t).map(|found_offs| byte_offs + found_offs)
  }

  while let Some(open) = find_next_char(s, '{', cur) {
    // Emit previous literal part
    let literal = &s[cur..open];
    if !literal.is_empty() {
      result.push(format!("\"{}\"", &escape(literal)));
    }

    // {}-enclosed substitution
    let open_post = open + '{'.len_utf8();
    if let Some(close) = find_next_char(s, '}', open_post) {
      let field = s[open_post..close].trim();
      if fields.iter().position(|s| s == field).is_none() {
        return Err(ExprError::NonexistentField(open_post, field.to_string()));
      }
      result.push(format!("row.at(\"{}\")", escape(field)));
      cur = close + '}'.len_utf8();
    } else {
      // Unmatched `{`: treat the rest as literal
      if open < s.len() {
        result.push(format!("\"{}\"", &escape(&s[open..])));
      }
      cur = s.len();
      break;
    }
  }

  // Emit trailing literal part
  if cur < s.len() {
    result.push(format!("\"{}\"", &escape(&s[cur..])));
  }

  Ok(if result.is_empty() {
    "\"\"".to_string()
  } else {
    result.join(" + ")
  })
}

impl model::Element {
  fn typst_source(&self, fields: &[String], is_preview: bool) -> Result<String, ExprError> {
    let mut args = Vec::new();

    if !is_preview {
      args.push(format!("{}pt, {}pt", self.x, self.y));
      if self.anchor != model::Anchor::default() {
        args.push(format!("anchor: {} + {}",
          match self.anchor.y {
            model::AnchorAxis::Min => "top",
            model::AnchorAxis::Cen => "horizon",
            model::AnchorAxis::Max => "bottom",
          },
          match self.anchor.x {
            model::AnchorAxis::Min => "left",
            model::AnchorAxis::Cen => "center",
            model::AnchorAxis::Max => "right",
          },
        ));
      }
      if self.scale != model::default_scale() {
        args.push(format!("scale: ({}, {})", self.scale.0, self.scale.1));
      }
      if self.rotation != 0.0 {
        args.push(format!("rotate: {}deg", self.rotation));
      }
    } else {
      args.push(format!("0pt, 0pt, in_flow: true"));
    }
    if let Some(w) = self.width {
      args.push(format!("width: {}pt", w));
    }
    if let Some(h) = self.height {
      args.push(format!("height: {}pt", h));
    }

    // Content
    let content = match &self.content {
      model::Content::Image { src } => {
        if is_preview {
          if let Ok(s) = expand_expr(src, &[]) {
            format!("#image({})", s)
          } else {
            format!("#raw(\"{}\")", escape(src))
          }
        } else {
          format!("#image({})", expand_expr(src, fields)?)
        }
      }
      model::Content::Text { family, size, weight, italic, tracking, text } => {
        let mut args = Vec::new();
        let families = family.iter().map(|s| format!("\"{}\"", s)).collect::<Vec<_>>().join(", ");
        args.push(format!("font: ({})", families));
        args.push(format!("size: {}pt", size));
        if *weight != model::Weight::default() {
          args.push(format!("weight: {}", Into::<i32>::into(*weight)));
        }
        if *italic {
          args.push(format!("style: \"italic\""));
        }
        if *tracking != 0.0 {
          args.push(format!("tracking: {}pt", tracking));
        }
        format!("#text({}, {})", args.join(", "),
          if is_preview { format!("\"{}\"", escape(text)) }
          else { expand_expr(text, fields)? })
      }
    };

    Ok(format!("#place-element({})[{}]", args.join(", "), content))
  }
}

impl model::Layout {
  fn typst_source(&self, csv_path: &str, fields: &[String]) -> Result<String, (usize, ExprError)> {
    let (page_width, page_height) = self.page;

    let elements_typst = self.elements.iter()
      .enumerate()
      .map(|(i, el)| el.typst_source(fields, false).map_err(|e| (i, e)))
      .collect::<Result<Vec<_>, _>>()?.join("\n  ");

    Ok(include_str!("support.typ").to_string() + format!(r#"
#set page(width: {page_width}pt, height: {page_height}pt, margin: 0pt)

#for row in csv("{}", row-type: dictionary) [
  {elements_typst}
  #pagebreak(weak: true)
]
"#, csv_path).as_str())
  }
}

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Debug)]
enum FindSingleError {
  Io(std::io::Error),
  NoneFound,
  MultipleFound,
}

fn find_single_ext<P: AsRef<Path>>(dir: P, ext: &[&str]) -> Result<PathBuf, FindSingleError> {
  let mut yml_files = std::fs::read_dir(&dir)
    .map_err(|e| FindSingleError::Io(e))?
    .flat_map(|entry| {
      if let Ok(entry) = entry {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false,
            |file_ext| ext.iter().any(|target_ext| file_ext == *target_ext)) {
          Some(path.strip_prefix(&dir).unwrap().to_path_buf())
        } else {
          None
        }
      } else {
        None
      }
    });

  let first = yml_files.next();
  if let Some(path) = first {
    if yml_files.next().is_none() {
      Ok(path)
    } else {
      Err(FindSingleError::MultipleFound)
    }
  } else {
    Err(FindSingleError::NoneFound)
  }
}

fn get_csv_fields<P: AsRef<Path>>(path: P) -> Result<Vec<String>, ProcessError> {
  let file = std::fs::File::open(path.as_ref())
    .map_err(|e| ProcessError::Io(path.as_ref().to_path_buf(), e))?;
  let mut reader = BufReader::new(file);
  let mut first_line = Vec::new();
  reader.read_until(0xa, &mut first_line)
    .map_err(|e| ProcessError::Io(path.as_ref().to_path_buf(), e))?;
  // TODO: Switch to `slice::strip_{prefix,suffix}` when they land in stable
  if first_line.ends_with(b"\n") {
    first_line.truncate(first_line.len() - 1);
  }
  if first_line.ends_with(b"\r") {
    first_line.truncate(first_line.len() - 1);
  }
  if first_line.starts_with(b"\xef\xbb\xbf") {
    first_line.drain(..3);
  }
  let first_line = str::from_utf8(&first_line)
    .map_err(|_e| ProcessError::CsvFieldsUtf8)?;
  let fields: Vec<String> =
    first_line.split(',').map(|s| s.to_string()).collect();
  let mut seen = std::collections::HashSet::new();
  for field in &fields {
    if field.is_empty() {
      continue;
    }
    if !seen.insert(field.as_str()) {
      return Err(ProcessError::CsvFieldsDuplicate(field.clone()));
    }
  }
  Ok(fields)
}

#[derive(Debug)]
enum ProcessError {
  FindYaml(FindSingleError),
  ParseYaml(serde_saphyr::Error),
  FindCsv(FindSingleError),
  CsvNameUtf8,
  CsvFieldsUtf8,
  CsvFieldsDuplicate(String),
  Io(std::path::PathBuf, std::io::Error),
  Expr(usize, ExprError),
  Render(render::RenderError),
}

fn process_pack_metadata<P: AsRef<Path>>(path: P, allow_empty_yaml: bool) -> Result<
  (PathBuf, model::Layout, PathBuf, Vec<String>),
  ProcessError>
{
  let (yaml_file_name, layout) = {
    let yaml_file_name = find_single_ext(&path, &["yml", "yaml"])
      .map_err(|e| ProcessError::FindYaml(e));
    if allow_empty_yaml &&
      let Err(ProcessError::FindYaml(FindSingleError::NoneFound)) = yaml_file_name
    {
      (PathBuf::from("layout.yml"), model::Layout {
        page: (1200., 1800.),
        elements: vec![],
      })
    } else {
      let yaml_file_name = yaml_file_name?;
      let yaml_path = path.as_ref().join(&yaml_file_name);
      let yaml = std::fs::read_to_string(&yaml_path)
        .map_err(|e| ProcessError::Io(yaml_path.to_path_buf(), e))?;

      let parse_result: Result<model::Layout, _> = serde_saphyr::from_str(&yaml);
      let layout = parse_result
        .map_err(|e| ProcessError::ParseYaml(e))?;
      (yaml_file_name, layout)
    }
  };

  let csv_file_name = find_single_ext(&path, &["csv"])
    .map_err(|e| ProcessError::FindCsv(e))?;
  let csv_path = path.as_ref().join(&csv_file_name);
  let list_fields = get_csv_fields(&csv_path)?;

  Ok((yaml_file_name, layout, csv_file_name, list_fields))
}

fn process_pack<P: AsRef<Path>, R, F: FnOnce(P, &str) -> Result<R, render::RenderError>>(path: P, layout: Option<&model::Layout>, render: F) -> Result<R, ProcessError> {
  let owned_layout;
  let (layout, csv_file_name, list_fields) =
    if let Some(layout) = layout {
      let (_, _, csv_file_name, list_fields) = process_pack_metadata(&path, false)?;
      (layout, csv_file_name, list_fields)
    } else {
      let (_, layout, csv_file_name, list_fields) = process_pack_metadata(&path, false)?;
      owned_layout = layout;
      (&owned_layout, csv_file_name, list_fields)
    };

  fn escape_path<P: AsRef<Path>>(path: P) -> Result<String, ProcessError> {
    Ok(path.as_ref().to_str().ok_or(ProcessError::CsvNameUtf8)?.to_string()
      .replace('\\', "\\\\").replace('"', "\\\""))
  }

  let typst_source = layout.typst_source(&escape_path(csv_file_name)?, &list_fields)
    .map_err(|(i, e)| ProcessError::Expr(i, e))?;

  let output = render(path, &typst_source)
    .map_err(|e| ProcessError::Render(e))?;
  Ok(output)
}

// Display

impl FindSingleError {
  fn message(&self) -> String {
    match self {
      FindSingleError::Io(e) => {
        f!("find-single-io-error", error => e.to_string())
      }
      FindSingleError::NoneFound => {
        f!("find-single-none-found")
      }
      FindSingleError::MultipleFound => {
        f!("find-single-multiple-found")
      }
    }
  }
}

impl ProcessError {
  fn message(&self) -> String {
    match self {
      ProcessError::FindYaml(e) => {
        f!("process-find-yaml", error => e.message())
      }
      ProcessError::ParseYaml(e) => {
        f!("process-parse-yaml", error => e.to_string())
      }
      ProcessError::FindCsv(e) => {
        f!("process-find-csv", error => e.message())
      }
      ProcessError::CsvNameUtf8 => {
        f!("process-csv-name-utf8")
      }
      ProcessError::CsvFieldsUtf8 => {
        f!("process-csv-fields-utf8")
      }
      ProcessError::CsvFieldsDuplicate(s) => {
        f!("process-csv-fields-duplicate", field => s.clone())
      }
      ProcessError::Io(path, e) => {
        f!("process-io",
          path => path.display().to_string(),
          error => e.to_string(),
        )
      }
      ProcessError::Expr(index, ExprError::NonexistentField(pos, name)) => {
        f!("process-expr-nonexistent-field",
          index => *index as i64,
          pos => *pos as i64,
          name => name.clone(),
        )
      }
      ProcessError::Render(render::RenderError::TypstCompilation(diags)) => {
        f!("process-render-typst-compilation",
          message => diags[0].message.to_string(),
          count => diags.len() as i64,
        )
      }
    }
  }
}

// CLI

use argh::{FromArgs, FromArgValue};

#[derive(Debug, FromArgs)]
#[argh(description = "A simplistic VPD (variable data printing) tool", help_triggers("-h", "--help"))]
struct Args {
  #[argh(positional)]
  pack_root: Option<String>,
  #[cfg_attr(
    feature = "gui",
    argh(option, short = 'o', description = "output file; GUI is started if not given")
  )]
  #[cfg_attr(
    not(feature = "gui"),
    argh(option, short = 'o', description = "output file")
  )]
  output: Option<String>,
  #[argh(option, short = 'f', description = "output format (pdf, pdf_seq, png_seq)", default = "Default::default()")]
  format: ArgsFormat,
  #[argh(switch, short = 'c', description = "compress output harder")]
  compress: bool,
}
#[derive(Debug, Clone, Default, FromArgValue)]
enum ArgsFormat {
  #[default]
  Pdf,
  PdfSeq,
  PngSeq,
}

fn process_and_write<P: AsRef<Path>, Q: AsRef<Path>>(pack_root: P, layout: Option<&model::Layout>, output: Q, format: ArgsFormat, compress: bool) -> Result<(), ProcessError> {
  match format {
    ArgsFormat::Pdf => {
      let bytes = process_pack(pack_root.as_ref(), layout, render::to_pdf)?;
      let p = output.as_ref();
      let bytes = opt_pdf::optimize(&bytes, compress);
      std::fs::write(&p, bytes).map_err(|e| ProcessError::Io(p.to_path_buf(), e))?;
    },
    ArgsFormat::PdfSeq => {
      let seq = process_pack(pack_root.as_ref(), layout, render::to_pdf_seq)?;
      let width = (seq.len().ilog10() + 1).max(3) as usize;
      for (i, bytes) in seq.into_iter().enumerate() {
        let mut p = output.as_ref().to_path_buf();
        p.push(format!("{:0width$}.pdf", i + 1));
        let bytes = opt_pdf::optimize(&bytes, compress);
        std::fs::write(&p, bytes).map_err(|e| ProcessError::Io(p, e))?;
      }
    },
    _ => (),
  }
  Ok(())
}

fn start_gui(root_path: Option<&str>) {
  #[cfg(feature = "gui")]
  {
    gui::start(root_path);
  }
  #[cfg(not(feature = "gui"))]
  {
    eprintln!("This version built without GUI feature >_<");
    eprintln!("Please specify <pack_root> and -o <output>");
  }
}

fn main() {
  localize::set_language(&sys_locale::get_locale().unwrap_or_else(|| "en".to_string()));

  let args: Args = argh::from_env();

  if let Some(pack_root) = args.pack_root {
    if let Some(output) = args.output {
      match process_and_write(&pack_root, None, &output, args.format, args.compress) {
        Err(e) => eprintln!("Error: {}", e.message()),
        Ok(_) => (),
      }
    } else {
      // TODO: Start GUI loaded with root
      start_gui(Some(&pack_root));
    }
  } else {
    // TODO: Start empty GUI
    start_gui(None);
  }
}
