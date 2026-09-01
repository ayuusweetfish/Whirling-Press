// Based on https://github.com/tfachmann/typst-as-library/blob/7710cc1/src/lib.rs

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt};
use typst_kit::fonts::FontStore;
use typst_pdf::PdfOptions;


pub struct TypstWorld {
  root: PathBuf,
  source: Source,
  library: LazyHash<Library>,
  fonts: FontStore,
  files: Arc<Mutex<HashMap<FileId, FileEntry>>>,
}

#[derive(Clone, Debug)]
struct FileEntry {
  bytes: Bytes,
  source: Option<Source>,
}

impl FileEntry {
  fn new(bytes: Vec<u8>, source: Option<Source>) -> Self {
    Self {
      bytes: Bytes::new(bytes),
      source,
    }
  }

  fn source(&mut self, id: FileId) -> FileResult<Source> {
    let source = if let Some(source) = &self.source {
      source
    } else {
      let text = std::str::from_utf8(&self.bytes).or(Err(FileError::InvalidUtf8))?;
      let text = text.trim_start_matches('\u{feff}');
      let source = Source::new(id, text.into());
      self.source.insert(source)
    };
    Ok(source.clone())
  }
}

impl TypstWorld {
  pub fn new(root: impl Into<PathBuf>, source: &str) -> Self {
    let root = root.into();

    let mut fonts = FontStore::new();
    fonts.extend(typst_kit::fonts::system());
    println!("{:?}", fonts.book());

    Self {
      library: LazyHash::new(Library::default()),
      root,
      fonts,
      source: Source::detached(source),
      files: Arc::new(Mutex::new(HashMap::new())),
    }
  }

  fn file(&self, id: FileId) -> FileResult<FileEntry> {
    let mut files = self.files.lock().or(Err(FileError::AccessDenied))?;
    if let Some(entry) = files.get(&id) {
      return Ok(entry.clone());
    }

    let path = id.vpath().realize(&self.root).or(Err(FileError::AccessDenied))?;
    let bytes = std::fs::read(&path).map_err(|e| FileError::from_io(e, &path))?;
    Ok(files
      .entry(id)
      .or_insert(FileEntry::new(bytes, None))
      .clone())
  }
}

impl typst::World for TypstWorld {
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
      self.file(id)?.source(id)
    }
  }

  fn file(&self, id: FileId) -> FileResult<Bytes> {
    self.file(id).map(|f| f.bytes.clone())
  }

  fn font(&self, id: usize) -> Option<Font> {
    self.fonts.font(id)
  }

  fn today(&self, _offset: Option<typst::foundations::Duration>) -> Option<Datetime> {
    None
  }
}

pub(crate) fn run() {
  let world = TypstWorld::new("../demo_pack", r#"
#set page(width: 759pt, height: 2663pt, margin: 0pt)
#place(top + left, dx: 0pt, dy: 174pt)[#image("qr.svg")]
#place(top + left, dx: 0pt, dy: 174pt)[#text(font: ("Source Code Pro", "Noto Serif"), size: 16pt, "AAAing 哈哈哈")]
"#);
  let document = typst::compile(&world).output.expect("Typst compilation failed");
  let pdf = typst_pdf::pdf(&document, &PdfOptions::default()).expect("PDF export failed");
  std::fs::write("/tmp/output.pdf", pdf).expect("File write failed");
  println!("Generated /tmp/output.pdf");
}
