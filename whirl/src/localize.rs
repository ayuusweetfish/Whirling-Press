use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

use fluent::{FluentBundle, FluentResource, FluentArgs, FluentError};
use fluent_langneg::LanguageIdentifier;

type Bundle = FluentBundle<FluentResource>;

struct Localization {
  bundles: HashMap<LanguageIdentifier, Bundle>,
}

thread_local! {
  static L: Localization = Localization::load();
}

static CURRENT_LANG: LazyLock<RwLock<LanguageIdentifier>> =
  LazyLock::new(|| RwLock::new("en".parse().unwrap()));

impl Localization {
  fn load() -> Self {
    let mut bundles = HashMap::new();

    let mut add_bundle = |lang: &str, src: &str| {
      let lang: LanguageIdentifier = lang.parse().unwrap();
      let res = FluentResource::try_new(src.to_string()).expect("bad FTL syntax");
      let mut bundle = FluentBundle::new(vec![lang.to_string().parse().unwrap()]);
      bundle.add_resource(res).expect("failed to add resource");
      bundles.insert(lang, bundle);
    };

    add_bundle("en", include_str!("../res/en.ftl"));
    add_bundle("zh-Hans", include_str!("../res/zh-Hans.ftl"));

    Localization { bundles }
  }
}

pub fn set_language(lang: &str) {
  let Ok(lang) = lang.parse::<LanguageIdentifier>() else { return };
  let available = L.with(|l| l.bundles.keys().cloned().collect::<Vec<_>>());
  let default = "en".parse().unwrap();
  let supported = fluent_langneg::negotiate_languages(
    &[lang],
    &available,
    Some(&default),
    fluent_langneg::NegotiationStrategy::Filtering,
  );
  *CURRENT_LANG.write().unwrap() = supported[0].clone();
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

#[macro_export]
macro_rules! f {
  ($id:expr) => {
    $crate::localize::format($id, &fluent::fluent_args!())
  };
  ($id:expr, $($key:ident => $value:expr),+ $(,)?) => {
    $crate::localize::format($id, &fluent::fluent_args!($(stringify!($key) => $value),+))
  };
}
