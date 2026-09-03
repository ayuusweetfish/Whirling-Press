mod model;
mod render;

#[derive(Debug)]
enum ExprError {
  NonexistentField(usize, String),
}

fn expand_expr(s: &str, fields: &[String]) -> Result<String, ExprError> {
  let mut result = Vec::new();
  let mut cur = 0;

  fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
  }

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
  fn typst_source(&self, fields: &[String]) -> Result<String, ExprError> {
    let mut args = Vec::new();

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
    if let Some(w) = self.width {
      args.push(format!("width: {}pt", w));
    }
    if let Some(h) = self.height {
      args.push(format!("height: {}pt", h));
    }
    if self.scale != model::default_scale() {
      args.push(format!("scale: ({}, {})", self.scale.0, self.scale.1));
    }
    if self.rotation != 0.0 {
      args.push(format!("rotate: {}deg", self.rotation));
    }

    // Content
    let content = match &self.content {
      model::Content::Image { src } => {
        format!("#image({})", expand_expr(src, fields)?)
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
        format!("#text({}, {})", args.join(", "), expand_expr(text, fields)?)
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
      .map(|(i, el)| el.typst_source(fields).map_err(|e| (i, e)))
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

fn get_csv_fields<P: AsRef<Path>>(path: P) -> std::io::Result<Vec<String>> {
  let file = std::fs::File::open(path)?;
  let mut reader = BufReader::new(file);
  let mut first_line = String::new();
  reader.read_line(&mut first_line)?;
  let fields = first_line.trim_end().split(',').map(|s| s.to_string()).collect();
  Ok(fields)
}

#[derive(Debug)]
enum ProcessError {
  FindYaml(FindSingleError),
  ParseYaml(serde_saphyr::Error),
  FindCsv(FindSingleError),
  CsvNameUtf8,
  Io(std::path::PathBuf, std::io::Error),
  Expr(usize, ExprError),
  Render(render::RenderError),
}

  fn process_pack<P: AsRef<Path>, R, F: FnOnce(P, &str) -> Result<R, render::RenderError>>(path: P, render: F) -> Result<R, ProcessError> {
  let yaml_file = find_single_ext(&path, &["yml", "yaml"])
    .map_err(|e| ProcessError::FindYaml(e))?;
  let p = path.as_ref().join(&yaml_file);
  let yaml = std::fs::read_to_string(&p)
    .map_err(|e| ProcessError::Io(p.to_path_buf(), e))?;

  let parse_result: Result<model::Layout, _> = serde_saphyr::from_str(&yaml);
  let layout = parse_result
    .map_err(|e| ProcessError::ParseYaml(e))?;

  let csv_file = find_single_ext(&path, &["csv"])
    .map_err(|e| ProcessError::FindCsv(e))?;
  let p = path.as_ref().join(&csv_file);
  let list_fields = get_csv_fields(&p)
    .map_err(|e| ProcessError::Io(p.to_path_buf(), e))?;

  fn escape_path<P: AsRef<Path>>(path: P) -> Result<String, ProcessError> {
    Ok(path.as_ref().to_str().ok_or(ProcessError::CsvNameUtf8)?.to_string()
      .replace('\\', "\\\\").replace('"', "\\\""))
  }

  let typst_source = layout.typst_source(&escape_path(csv_file)?, &list_fields)
    .map_err(|(i, e)| ProcessError::Expr(i, e))?;

  let output = render(path, &typst_source)
    .map_err(|e| ProcessError::Render(e))?;
  Ok(output)
}

use argh::{FromArgs, FromArgValue};

#[derive(Debug, FromArgs)]
#[argh(description = "A simplistic VPD (variable data printing) tool", help_triggers("-h", "--help"))]
struct Args {
  #[argh(positional)]
  pack_root: Option<String>,
  #[argh(option, short = 'o', description = "output file; GUI is started if not given")]
  output: Option<String>,
  #[argh(option, short = 'f', description = "output format (pdf, pdf_seq, png_seq)", default = "Default::default()")]
  format: ArgsFormat,
}
#[derive(Debug, Clone, Default, FromArgValue)]
enum ArgsFormat {
  #[default]
  Pdf,
  PdfSeq,
  PngSeq,
}

fn process_and_write(pack_root: &str, output: &str, format: ArgsFormat) -> Result<(), ProcessError> {
  match format {
    ArgsFormat::Pdf => {
      let bytes = process_pack(&pack_root, render::to_pdf)?;
      let p = Path::new(&output);
      std::fs::write(&p, bytes).map_err(|e| ProcessError::Io(p.to_path_buf(), e))?;
    },
    ArgsFormat::PdfSeq => {
      let seq = process_pack(&pack_root, render::to_pdf_seq)?;
      let width = (seq.len().ilog10() + 1).max(3) as usize;
      for (i, bytes) in seq.into_iter().enumerate() {
        let mut p = Path::new(&output).to_path_buf();
        p.push(format!("{:0width$}.pdf", i + 1));
        std::fs::write(&p, bytes).map_err(|e| ProcessError::Io(p, e))?;
      }
    },
    _ => (),
  }
  Ok(())
}

fn main() {
  let args: Args = argh::from_env();
  println!("{:?}", args);

  if let Some(pack_root) = args.pack_root {
    if let Some(output) = args.output {
      match process_and_write(&pack_root, &output, args.format) {
        Err(e) => println!("{:?}", e),
        Ok(_) => (),
      }
    } else {
      // TODO: Start GUI loaded with root
    }
  } else {
    // TODO: Start empty GUI
  }
}
