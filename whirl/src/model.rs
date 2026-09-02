use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Layout {
  pub page: (f32, f32),
  pub elements: Vec<Element>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Element {
  pub x: f32,
  pub y: f32,
  #[serde(default, skip_serializing_if = "is_default")]
  pub anchor: Anchor,
  #[serde(default = "default_scale", skip_serializing_if = "is_default_scale")]
  pub scale: (f32, f32),
  #[serde(default, skip_serializing_if = "is_default")]
  pub rotation: f32,
  #[serde(default, skip_serializing_if = "is_default")]
  pub width: Option<f32>,
  #[serde(default, skip_serializing_if = "is_default")]
  pub height: Option<f32>,
  pub content: Content,
}

#[derive(Debug, Deserialize, Serialize, Default, PartialEq)]
#[serde(try_from = "(f32, f32)")]
pub struct Anchor {
  pub x: AnchorAxis,
  pub y: AnchorAxis,
}

#[derive(Debug, Deserialize, Serialize, Default, PartialEq)]
pub enum AnchorAxis {
  #[default]
  Min,
  Cen,
  Max,
}
impl TryFrom<f32> for AnchorAxis {
  type Error = String;
  fn try_from(value: f32) -> Result<Self, Self::Error> {
    match value {
      0.0 => Ok(AnchorAxis::Min),
      0.5 => Ok(AnchorAxis::Cen),
      1.0 => Ok(AnchorAxis::Max),
      _ => Err("Anchor coordinate value must be one of 0, 0.5, or 1".to_string()),
    }
  }
}
impl TryFrom<(f32, f32)> for Anchor {
  type Error = String;
  fn try_from((x, y): (f32, f32)) -> Result<Self, Self::Error> {
    Ok(Anchor {
      x: x.try_into()?,
      y: y.try_into()?,
    })
  }
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
