// mod render;

mod model;

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
        format!("#image(\"{}\")", src)
      }
      model::Content::Text { family, size, bold, italic, tracking, text } => {
        let mut args = Vec::new();
        let families = family.iter().map(|s| format!("\"{}\"", s)).collect::<Vec<_>>().join(", ");
        args.push(format!("font: ({})", families));
        args.push(format!("size: {}pt", size));
        if *bold {
          args.push(format!("weight: \"bold\""));
        }
        if *italic {
          args.push(format!("style: \"italic\""));
        }
        if *tracking != 0.0 {
          args.push(format!("tracking: {}pt", tracking));
        }
        format!("#text({}, \"{}\")", args.join(", "), text)
      }
    };

    format!("#place-element({})[{}]", args.join(", "), content)
  }
}

impl model::Layout {
  fn typst_source(&self) -> String {
    let (page_width, page_height) = self.page;

    let elements_typst = self.elements.iter().map(|e| e.typst_source())
      .collect::<Vec<_>>().join("\n  ");

    include_str!("support.typ").to_string() + format!(r#"
#set page(width: {page_width}pt, height: {page_height}pt, margin: 0pt)

#for row in csv("list.csv", row-type: dictionary) [
  #let (n, img_a, img_b, sn, order_id, qty, name, addr) = row
  {elements_typst}
  #pagebreak(weak: true)
]
"#).as_str()
  }
}

fn main_0() {
  let yaml = r#"
page: [759, 2663]
elements:
- { x: 0, y: 174, content: !image { src: "{{img_a}}" } }
- { x: 0, y: 0, content: !image { src: frame.pdf } }
- { x: 740, y: 2631, anchor: [1, 1], width: 110, content: !image { src: qr.svg } }
- { x: 250, y: 1872.5, anchor: [1, 1], content: !text { family: [Instrument Sans, Noto Sans CJK SC], size: 16, italic: true, tracking: -0.5, text: Order ID } }
- { x: 252, y: 1872, anchor: [0, 1], content: !text { family: [Instrument Sans, Noto Sans CJK SC], size: 16, text: ":" } }
- { x: 271, y: 1872.5, anchor: [0, 1], content: !text { family: [Instrument Sans], size: 16, text: "{{order_id}}" } }
"#;

  let parse_result: Result<model::Layout, _> = serde_saphyr::from_str(yaml);
  let layout = parse_result.expect("parse failed");

  let typst_source = layout.typst_source();
  println!("{}", typst_source);
}

fn main() {
  // render::run();
  main_0();
}
