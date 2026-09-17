use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Layout {
  pub page: (f32, f32),
  pub elements: Vec<Element>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
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

impl Element {
  pub fn from_content(content: Content) -> Self {
    Self {
      x: 0.0,
      y: 0.0,
      anchor: Default::default(),
      scale: default_scale(),
      rotation: Default::default(),
      width: Default::default(),
      height: Default::default(),
      content,
    }
  }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Copy)]
#[serde(try_from = "(f32, f32)", into = "(f32, f32)")]
pub struct Anchor {
  pub x: AnchorAxis,
  pub y: AnchorAxis,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Copy)]
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
impl From<AnchorAxis> for f32 {
  fn from(value: AnchorAxis) -> Self {
    match value {
      AnchorAxis::Min => 0.0,
      AnchorAxis::Cen => 0.5,
      AnchorAxis::Max => 1.0,
    }
  }
}
impl From<Anchor> for (f32, f32) {
  fn from(anchor: Anchor) -> Self {
    (anchor.x.into(), anchor.y.into())
  }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Content {
  #[serde(rename = "image")]
  Image {
    src: String,
  },
  #[serde(rename = "text")]
  Text {
    family: Vec<String>,
    size: f32,
    #[serde(default, skip_serializing_if = "is_default")]
    weight: Weight,
    #[serde(default, skip_serializing_if = "is_default")]
    italic: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    tracking: f32,
    text: String,
  },
}
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Copy)]
#[serde(try_from = "f32", into = "i32")]
pub enum Weight {
  Thin,
  ExtraLight,
  Light,
  #[default]
  Regular,
  Medium,
  SemiBold,
  Bold,
  ExtraBold,
  Black,
}
impl TryFrom<f32> for Weight {
  type Error = String;
  fn try_from(value: f32) -> Result<Self, Self::Error> {
    match value {
      100.0 => Ok(Weight::Thin),
      200.0 => Ok(Weight::ExtraLight),
      300.0 => Ok(Weight::Light),
      400.0 => Ok(Weight::Regular),
      500.0 => Ok(Weight::Medium),
      600.0 => Ok(Weight::SemiBold),
      700.0 => Ok(Weight::Bold),
      800.0 => Ok(Weight::ExtraBold),
      900.0 => Ok(Weight::Black),
      _ => Err("Weight value must be one of 100, 200, ..., 900".to_string()),
    }
  }
}
impl Into<i32> for Weight {
  fn into(self) -> i32 {
    match self {
      Weight::Thin => 100,
      Weight::ExtraLight => 200,
      Weight::Light => 300,
      Weight::Regular => 400,
      Weight::Medium => 500,
      Weight::SemiBold => 600,
      Weight::Bold => 700,
      Weight::ExtraBold => 800,
      Weight::Black => 900,
    }
  }
}

pub fn default_scale() -> (f32, f32) { (1.0, 1.0) }
fn is_default_scale(v: &(f32, f32)) -> bool { *v == default_scale() }

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
  *v == T::default()
}
