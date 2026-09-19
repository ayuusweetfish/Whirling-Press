use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

use fluent::{FluentBundle, FluentResource, FluentArgs, FluentError};
use unic_langid::{langid, LanguageIdentifier};

type Bundle = FluentBundle<FluentResource>;

struct Localization {
  bundles: HashMap<LanguageIdentifier, Bundle>,
}

thread_local! {
  static L: Localization = Localization::load();
}

static CURRENT_LANG: LazyLock<RwLock<LanguageIdentifier>> =
  LazyLock::new(|| RwLock::new(langid!("en")));

impl Localization {
  fn load() -> Self {
    let mut bundles = HashMap::new();

    let mut add_bundle = |lang: LanguageIdentifier, src: &str| {
      let res = FluentResource::try_new(src.to_string()).expect("bad FTL syntax");
      let mut bundle = FluentBundle::new(vec![lang.clone()]);
      bundle.add_resource(res).expect("failed to add resource");
      bundles.insert(lang, bundle);
    };

    add_bundle(langid!("en"), include_str!("../res/en.ftl"));
    add_bundle(langid!("zh-Hans"), include_str!("../res/zh-Hans.ftl"));

    Localization { bundles }
  }
}

pub fn set_language(lang: &str) -> bool {
  let Ok(lang) = lang.parse::<LanguageIdentifier>() else { return false };
  if L.with(|l| l.bundles.contains_key(&lang)) {
    *CURRENT_LANG.write().unwrap() = lang;
    true
  } else {
    false
  }
}

pub fn current_language() -> LanguageIdentifier {
  CURRENT_LANG.read().unwrap().clone()
}

pub fn format(id: &str, args: &FluentArgs) -> String {
  L.with(|l| {
    let bundle = l.bundles.get(&current_language()).unwrap();
    let Some(msg) = bundle.get_message(id) else { return id.to_string() };
    let Some(pattern) = msg.value() else { return id.to_string() };
    let mut errors: Vec<FluentError> = Vec::new();
    let formatted = bundle.format_pattern(pattern, Some(args), &mut errors);
    debug_assert!(errors.is_empty(), "Fluent errors: {errors:?}");
    formatted.to_string()
  })
}
