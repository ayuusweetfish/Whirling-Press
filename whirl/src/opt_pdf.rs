use lopdf::{Dictionary, Document, Object, ObjectId, StringFormat};

use std::collections::HashMap;
use std::collections::hash_map::{DefaultHasher, Entry};
use std::hash::{Hash, Hasher};

// Byte content and dictionary, with order-independent hash
struct StreamKey<'a> {
  content: &'a [u8],
  dict: &'a Dictionary,
}
impl PartialEq for StreamKey<'_> {
  fn eq(&self, other: &Self) -> bool {
    self.content == other.content && self.dict == other.dict
  }
}
impl Eq for StreamKey<'_> {}
impl Hash for StreamKey<'_> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.content.hash(state);
    hash_dict(self.dict, state);
  }
}
fn hash_dict<H: Hasher>(dict: &Dictionary, state: &mut H) {
  let mut combined = dict.len() as u64;
  for (key, value) in dict.iter() {
    let mut entry = DefaultHasher::new();
    key.hash(&mut entry);
    hash_object(value, &mut entry);
    combined ^= entry.finish();
  }
  combined.hash(state);
}
fn hash_object<H: Hasher>(object: &Object, state: &mut H) {
  match object {
    Object::Null => {
      state.write_u8(0);
    }
    Object::Boolean(value) => {
      state.write_u8(1);
      value.hash(state);
    }
    Object::Integer(value) => {
      state.write_u8(2);
      value.hash(state);
    }
    Object::Real(value) => {
      state.write_u8(3);
      // Normalize; see f32::total_cmp
      let x = value.to_bits();
      let x = x ^ (((x as i32 >> 31) as u32) >> 1);
      x.hash(state);
    }
    Object::Name(name) => {
      state.write_u8(4);
      name.hash(state);
    }
    Object::String(bytes, format) => {
      state.write_u8(5);
      bytes.hash(state);
      state.write_u8(match format {
        StringFormat::Literal => 0,
        StringFormat::Hexadecimal => 1,
      });
    }
    Object::Array(array) => {
      state.write_u8(6);
      array.len().hash(state);
      for element in array {
        hash_object(element, state);
      }
    }
    Object::Dictionary(dict) => {
      state.write_u8(7);
      hash_dict(dict, state);
    }
    Object::Stream(stream) => {
      state.write_u8(8);
      hash_dict(&stream.dict, state);
      stream.content.hash(state);
    }
    Object::Reference(id) => {
      state.write_u8(9);
      id.hash(state);
    }
  }
}

pub(crate) fn try_optimize(bytes: &[u8], recompress: bool) -> Option<Vec<u8>> {
  let mut doc = Document::load_mem(bytes).ok()?;

  let mut canonical: HashMap<StreamKey, ObjectId> = HashMap::new();
  // Replacement map: duplicate -> canonical
  let mut replacements: HashMap<ObjectId, ObjectId> = HashMap::new();

  for (object_id, object) in &doc.objects {
    if let Object::Stream(stream) = object {
      let key = StreamKey { content: &stream.content, dict: &stream.dict };
      match canonical.entry(key) {
        Entry::Occupied(slot) => { replacements.insert(*object_id, *slot.get()); }
        Entry::Vacant(slot) => { slot.insert(*object_id); }
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
  if recompress {
    doc.decompress();
    doc.compress();
  }

  let mut buf = Vec::<u8>::new();
  doc.save_modern(&mut buf).ok()?;
  Some(buf)
}

pub(crate) fn optimize(bytes: &[u8], recompress: bool) -> Vec<u8> {
  try_optimize(&bytes, recompress).unwrap_or_else(|| bytes.to_vec())
}
