use lopdf::{Document, Object, ObjectId};

use std::collections::HashMap;

pub(crate) fn try_optimize(bytes: &[u8]) -> Option<Vec<u8>> {
  let mut doc = Document::load_mem(bytes).ok()?;
  let mut stream_map: HashMap<&[u8], Vec<ObjectId>> = HashMap::new();

  for (object_id, object) in &doc.objects {
    if let Object::Stream(stream) = object {
      stream_map.entry(&stream.content[..]).or_default().push(*object_id);
    }
  }

  // Replacement map: duplicate -> canonical
  let mut replacements = HashMap::new();
  for group in stream_map.values() {
    if group.len() > 1 {
      let canon = group[0];
      for &dup in group.iter().skip(1) {
        replacements.insert(dup, canon);
      }
    }
  }

  // Recursively replace all references
  fn replace_references(obj: &mut Object, replacements: &HashMap<ObjectId, ObjectId>) {
    match obj {
      Object::Dictionary(dict) => {
        for (_, value) in dict.iter_mut() {
          replace_references(value, replacements);
        }
      }
      Object::Array(arr) => {
        for elem in arr.iter_mut() {
          replace_references(elem, replacements);
        }
      }
      Object::Stream(stream) => {
        for (_, value) in stream.dict.iter_mut() {
          replace_references(value, replacements);
        }
      }
      Object::Reference(id) => {
        if let Some(&canon) = replacements.get(id) {
          *obj = Object::Reference(canon);
        }
      }
      _ => ()
    }
  }
  for (_, obj) in doc.objects.iter_mut() {
    replace_references(obj, &replacements);
  }
  let mut trailer = Object::Dictionary(doc.trailer.clone());
  replace_references(&mut trailer, &replacements);
  if let Object::Dictionary(trailer) = trailer {
    doc.trailer = trailer;
  }

  // Remove duplicate (now dangling) objects
  for id in replacements.keys().copied() {
    doc.objects.remove(&id);
  }

  // Re-compress everything for the most space savings!
  if false {
    doc.decompress();
    doc.compress();
  }

  let mut buf = Vec::<u8>::new();
  doc.save_modern(&mut buf).ok()?;
  Some(buf)
}

pub(crate) fn optimize(bytes: &[u8]) -> Vec<u8> {
  try_optimize(&bytes).unwrap_or_else(|| bytes.to_vec())
}
