// Based on https://github.com/tfachmann/typst-as-library/blob/7710cc1/src/lib.rs

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt};
use typst_kit::fonts::FontStore;
use typst_pdf::PdfOptions;


pub struct SimpleWorld {
  root: PathBuf,
  source: Source,
  library: LazyHash<Library>,
  fonts: FontStore,
  files: Arc<RwLock<HashMap<FileId, Bytes>>>,
    // trait `typst::World` requires `Sync`
}

impl SimpleWorld {
  pub fn new(root: impl Into<PathBuf>, source: &str) -> Self {
    let mut fonts = FontStore::new();
    fonts.extend(typst_kit::fonts::system());

    Self {
      library: LazyHash::new(Library::default()),
      root: root.into(),
      fonts,
      source: Source::detached(source),
      files: Arc::new(RwLock::new(HashMap::new())),
    }
  }
}

impl typst::World for SimpleWorld {
  fn library(&self) -> &LazyHash<Library> {
    &self.library
  }

  fn book(&self) -> &LazyHash<FontBook> {
    self.fonts.book()
  }

  fn main(&self) -> FileId {
    self.source.id()
  }

  fn source(&self, id: FileId) -> FileResult<Source> {
    if id == self.source.id() {
      Ok(self.source.clone())
    } else {
      Err(FileError::NotSource)
    }
  }

  fn file(&self, id: FileId) -> FileResult<Bytes> {
    if let Some(bytes) = self.files.read().unwrap().get(&id) {
      return Ok(bytes.clone());
    }
    let path = id.vpath().realize(&self.root).or(Err(FileError::AccessDenied))?;
    let bytes_u8 = std::fs::read(&path).map_err(|e| FileError::from_io(e, &path))?;
    Ok(self.files.write().unwrap().entry(id).or_insert(Bytes::new(bytes_u8)).clone())
  }

  fn font(&self, id: usize) -> Option<Font> {
    self.fonts.font(id)
  }

  fn today(&self, _offset: Option<typst::foundations::Duration>) -> Option<Datetime> {
    None
  }
}

pub(crate) fn run() {
  let world = SimpleWorld::new("../demo_pack", r#"
#set page(width: 759pt, height: 2663pt, margin: 0pt)
#place(top + left, dx: 0pt, dy: 174pt)[#image("qr.svg")]
#place(top + left, dx: 0pt, dy: 174pt)[#text(font: ("Source Code Pro", "Noto Serif"), size: 16pt, "AAAing 哈哈哈")]
"#);
  let document = typst::compile(&world).output.expect("Typst compilation failed");
  let pdf = typst_pdf::pdf(&document, &PdfOptions::default()).expect("PDF export failed");
  std::fs::write("/tmp/output.pdf", pdf).expect("File write failed");
  println!("Generated /tmp/output.pdf");
}
