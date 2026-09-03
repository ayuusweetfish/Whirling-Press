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
use typst::model::Document;
use typst_layout::PagedDocument;

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
    let root = root.into();

    let mut fonts = FontStore::new();
    fonts.extend(typst_kit::fonts::scan(&root));
    fonts.extend(typst_kit::fonts::system());

    Self {
      library: LazyHash::new(Library::default()),
      root,
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

#[derive(Debug)]
pub(crate) enum RenderError {
  TypstCompilation(Vec<typst::diag::SourceDiagnostic>),
  TypstPdfOutput(Vec<typst::diag::SourceDiagnostic>),
}

use crate::opt_pdf;

pub(crate) fn to_pdf<P: AsRef<std::path::Path>>(root: P, source: &str) -> Result<Vec<u8>, RenderError> {
  let world = SimpleWorld::new(root.as_ref(), source);
  let doc = typst::compile(&world).output
    .map_err(|e| RenderError::TypstCompilation(e.to_vec()))?;
  let pdf_bytes = typst_pdf::pdf(&doc, &PdfOptions::default())
    .map_err(|e| RenderError::TypstCompilation(e.to_vec()))?;
  let pdf_bytes = opt_pdf::optimize(&pdf_bytes);
  Ok(pdf_bytes)
}

pub(crate) fn to_pdf_seq<P: AsRef<std::path::Path>>(root: P, source: &str) -> Result<Vec<Vec<u8>>, RenderError> {
  let world = SimpleWorld::new(root.as_ref(), source);
  let doc = typst::compile::<PagedDocument>(&world).output
    .map_err(|e| RenderError::TypstCompilation(e.to_vec()))?;
  let pdf_seq_bytes = doc.pages().iter().map(|p| {
    let doc = PagedDocument::new([p.clone()].into(), doc.info().clone());
    let pdf_bytes = typst_pdf::pdf(&doc, &PdfOptions::default())
      .map_err(|e| RenderError::TypstCompilation(e.to_vec()))?;
    let pdf_bytes = opt_pdf::optimize(&pdf_bytes);
    Ok::<_, RenderError>(pdf_bytes)
  }).collect::<Result<_, _>>()?;
  Ok(pdf_seq_bytes)
}
