// mod render;

mod model;

fn main_0() {
  let yaml = r#"
page: [759, 2663]
items:
- { x: 0, y: 174, content: !image { src: "{{img_a}}" } }
- { x: 0, y: 0, content: !image { src: frame.pdf } }
- { x: 740, y: 2631, anchor: [1, 1], width: 110, content: !image { src: qr.svg } }
- { x: 250, y: 1872.5, anchor: [1, 1], content: !text { family: [Instrument Sans, Noto Sans CJK SC], size: 16, italic: true, tracking: -0.5, text: Order ID } }
- { x: 252, y: 1872, anchor: [0, 1], content: !text { family: [Instrument Sans, Noto Sans CJK SC], size: 16, text: ":" } }
- { x: 250, y: 1872.5, anchor: [0, 1], content: !text { text: "{{order_id}}" } }
"#;

  let parse_result: Result<model::Layout, _> = serde_saphyr::from_str(yaml);
  let items = parse_result.expect("parse failed");
  println!("{:?}", items);

  let options = serde_saphyr::ser_options! {
    compact_list_indent: true,
    yaml_12: true,
  };
  println!("{}", serde_saphyr::to_string_with_options(&items, options.clone()).unwrap());
  println!("{:?}", serde_saphyr::from_str::<model::Layout>(&serde_saphyr::to_string_with_options(&items, options.clone()).unwrap()).unwrap());
}

fn main() {
  // render::run();
  main_0();
}
