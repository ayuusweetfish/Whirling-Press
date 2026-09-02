// mod render;

mod model;

fn expand_expr(s: &str) -> String {
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
      result.push(format!("({})", s[open_post..close].trim()));
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

  if result.is_empty() {
    "\"\"".to_string()

  // Simplify cases with only one variable (XXX: doesn't handle higher codepoints for now!)
  } else if result.len() == 1 && result[0].bytes().enumerate().all(|(i, b)| {
    if i == 0 { b == b'(' }
    else if i == result[0].len() - 1 { b == b')' }
    else {
      let c = b as char;
      c.is_alphanumeric() || c == '_' || c == '-'
    }
  }) {
    result[0][1..(result[0].len() - 1)].to_string()
  } else {
    result.join(" + ")
  }
}

impl model::Element {
  fn typst_source(&self) -> String {
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
        format!("#image({})", expand_expr(src))
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
        format!("#text({}, {})", args.join(", "), expand_expr(text))
      }
    };

    format!("#place-element({})[{}]", args.join(", "), content)
  }
}

impl model::Layout {
  fn typst_source(&self, fields: &[String]) -> String {
    let (page_width, page_height) = self.page;

    let elements_typst = self.elements.iter().map(|e| e.typst_source())
      .collect::<Vec<_>>().join("\n  ");

    include_str!("support.typ").to_string() + format!(r#"
#set page(width: {page_width}pt, height: {page_height}pt, margin: 0pt)

#for ({}) in csv("list.csv", row-type: dictionary) [
  {elements_typst}
  #pagebreak(weak: true)
]
"#, fields.join(", ")).as_str()
  }
}

use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

fn get_csv_fields<P: AsRef<Path>>(path: P) -> io::Result<Vec<String>> {
  let file = File::open(path)?;
  let mut reader = BufReader::new(file);
  let mut first_line = String::new();
  reader.read_line(&mut first_line)?;
  let fields = first_line.trim_end().split(',').map(|s| s.to_string()).collect();
  Ok(fields)
}

fn main_0() {
  let yaml = include_str!("../../demo_pack/layout.yml");

  let parse_result: Result<model::Layout, _> = serde_saphyr::from_str(yaml);
  let layout = parse_result.expect("parse failed");

  let list_fields = get_csv_fields("../demo_pack/list.csv").expect("Read CSV failed");

  let typst_source = layout.typst_source(&list_fields);
  println!("{}", typst_source);
}

fn main() {
  // render::run();
  main_0();
}
