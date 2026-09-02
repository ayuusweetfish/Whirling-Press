use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Layout {
  page: (f32, f32),
  items: Vec<Item>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Item {
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
pub enum Content {
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
