use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct Item {
  x: f32,
  y: f32,
  #[serde(default, skip_serializing_if = "is_default")]
  anchor: (f32, f32),
  #[serde(default = "default_scale", skip_serializing_if = "is_default_scale")]
  scale: (f32, f32),
  #[serde(default, skip_serializing_if = "is_default")]
  rotation: f32,
  #[serde(default, skip_serializing_if = "is_default")]
  width: Option<f32>,
  #[serde(default, skip_serializing_if = "is_default")]
  height: Option<f32>,
  content: Content,
}

#[derive(Debug, Deserialize, Serialize)]
enum Content {
  #[serde(rename = "image")]
  Image {
    src: String,
  },
  #[serde(rename = "text")]
  Text {
    #[serde(default, skip_serializing_if = "is_default")]
    family: Vec<String>,
    #[serde(default = "default_text_size", skip_serializing_if = "is_default_text_size")]
    size: f32,
    #[serde(default, skip_serializing_if = "is_default")]
    bold: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    italic: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    tracking: f32,
    text: String,
  },
}

fn default_scale() -> (f32, f32) { (1.0, 1.0) }
fn is_default_scale(v: &(f32, f32)) -> bool { *v == default_scale() }

fn default_text_size() -> f32 { 16.0 }
fn is_default_text_size(v: &f32) -> bool { *v == default_text_size() }

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
  *v == T::default()
}

fn main() {
  let yaml = r#"
- { x: 0, y: 174, content: !image { src: "{{img_a}}" } }
- { x: 0, y: 0, content: !image { src: frame.pdf } }
- { x: 740, y: 2631, anchor: [1, 1], width: 110, content: !image { src: qr.svg } }
- { x: 250, y: 1872.5, anchor: [1, 1], content: !text { family: [Instrument Sans, Noto Sans CJK SC], size: 16, italic: true, tracking: -0.5, text: Order ID } }
- { x: 252, y: 1872, anchor: [0, 1], content: !text { family: [Instrument Sans, Noto Sans CJK SC], size: 16, text: ":" } }
- { x: 250, y: 1872.5, anchor: [0, 1], content: !text { text: "{{order_id}}" } }
"#;

  let parse_result: Result<Vec<Item>, _> = serde_saphyr::from_str(yaml);
  let items = parse_result.expect("parse failed");
  println!("{:?}", items);

  let options = serde_saphyr::ser_options! {
    compact_list_indent: true,
    yaml_12: true,
  };
  println!("{}", serde_saphyr::to_string_with_options(&items, options.clone()).unwrap());
  println!("{:?}", serde_saphyr::from_str::<Vec<Item>>(&serde_saphyr::to_string_with_options(&items, options.clone()).unwrap()).unwrap());
}
