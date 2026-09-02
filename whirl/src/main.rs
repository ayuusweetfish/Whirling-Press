// mod render;

mod model;

impl model::Element {
  fn typst_source(&self) -> String {
    let (x, y) = (self.x, self.y);
    let vars = "".to_string();  // TODO
    let element_source = "#image(\"1.svg\")".to_string(); // TODO
    format!("#place-element({x}pt, {y}pt{vars})[{element_source}]")
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
- { x: 250, y: 1872.5, anchor: [0, 1], content: !text { text: "{{order_id}}" } }
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
