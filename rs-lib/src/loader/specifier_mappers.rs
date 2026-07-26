// Copyright 2018-2024 the Deno authors. MIT license.

use deno_ast::ModuleSpecifier;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::PackageMappedSpecifier;

pub trait SpecifierMapper {
  fn map(&self, specifier: &ModuleSpecifier) -> Option<PackageMappedSpecifier>;

  /// Maps a declaration file specifier to the npm package that provides it
  /// (ex. an `@types/` package served by a cdn).
  fn map_types(
    &self,
    _specifier: &ModuleSpecifier,
  ) -> Option<PackageMappedSpecifier> {
    None
  }
}

/// Gets the npm package that provides the declaration file at the specifier.
pub fn get_types_package_for_specifier(
  mappers: &[Box<dyn SpecifierMapper>],
  specifier: &ModuleSpecifier,
) -> Option<PackageMappedSpecifier> {
  mappers
    .iter()
    .find_map(|mapper| mapper.map_types(specifier))
}

pub fn get_all_specifier_mappers() -> Vec<Box<dyn SpecifierMapper>> {
  vec![
    Box::new(DenoStdNodeSpecifierMapper::new("assert")),
    Box::new(DenoStdNodeSpecifierMapper::new("assert/strict")),
    Box::new(DenoStdNodeSpecifierMapper::new("buffer")),
    Box::new(DenoStdNodeSpecifierMapper::new("console")),
    Box::new(DenoStdNodeSpecifierMapper::new("constants")),
    Box::new(DenoStdNodeSpecifierMapper::new("crypto")),
    Box::new(DenoStdNodeSpecifierMapper::new("child_process")),
    Box::new(DenoStdNodeSpecifierMapper::new("dns")),
    Box::new(DenoStdNodeSpecifierMapper::new("events")),
    Box::new(DenoStdNodeSpecifierMapper::new("fs")),
    Box::new(DenoStdNodeSpecifierMapper::new("fs/promises")),
    Box::new(DenoStdNodeSpecifierMapper::new("http")),
    Box::new(DenoStdNodeSpecifierMapper::new("module")),
    Box::new(DenoStdNodeSpecifierMapper::new("net")),
    Box::new(DenoStdNodeSpecifierMapper::new("os")),
    Box::new(DenoStdNodeSpecifierMapper::new("path")),
    Box::new(DenoStdNodeSpecifierMapper::new("perf_hooks")),
    Box::new(DenoStdNodeSpecifierMapper::new("process")),
    Box::new(DenoStdNodeSpecifierMapper::new("querystring")),
    Box::new(DenoStdNodeSpecifierMapper::new("readline")),
    Box::new(DenoStdNodeSpecifierMapper::new("stream")),
    Box::new(DenoStdNodeSpecifierMapper::new("string_decoder")),
    Box::new(DenoStdNodeSpecifierMapper::new("sys")),
    Box::new(DenoStdNodeSpecifierMapper::new("timers")),
    Box::new(DenoStdNodeSpecifierMapper::new("timers/promises")),
    Box::new(DenoStdNodeSpecifierMapper::new("tty")),
    Box::new(DenoStdNodeSpecifierMapper::new("url")),
    Box::new(DenoStdNodeSpecifierMapper::new("util")),
    Box::new(DenoStdNodeSpecifierMapper::new("worker_threads")),
    Box::new(SkypackMapper),
    Box::new(EsmShMapper),
  ]
}

// good enough for a first pass
static SKYPACK_MAPPING_RE: Lazy<Regex> = Lazy::new(|| {
  Regex::new(
    r"^https://cdn\.skypack\.dev/(\-/)?(@?[^@?]+)@([0-9.\^~\-A-Za-z]+)(?:/([^#?]+))?",
  )
  .unwrap()
});
static ESMSH_MAPPING_RE: Lazy<Regex> = Lazy::new(|| {
  Regex::new(
    r"^https://esm\.sh/(v\d+/)?(@?[^@?]+)@([0-9.\^~\-A-Za-z]+)(?:/([^#?]+))?$",
  )
  .unwrap()
});

struct SkypackMapper;

impl SpecifierMapper for SkypackMapper {
  fn map(&self, specifier: &ModuleSpecifier) -> Option<PackageMappedSpecifier> {
    if specifier.path().starts_with("/-/") {
      // ignore, it's an internal url
      return None;
    }

    let captures = SKYPACK_MAPPING_RE.captures(specifier.as_str())?;
    let sub_path = captures.get(4).map(|m| m.as_str().to_owned());

    // don't use the package for declaration file imports
    if let Some(sub_path) = &sub_path {
      // todo(dsherret): this should probably work on media type
      if sub_path.to_lowercase().ends_with(".d.ts") {
        return None;
      }
    }

    let name = captures.get(2).unwrap().as_str().to_string();
    let version = captures
      .get(3)
      .unwrap()
      .as_str()
      .trim_start_matches('v')
      .to_string();

    Some(PackageMappedSpecifier {
      name,
      version: Some(version),
      sub_path,
      peer_dependency: false,
    })
  }

  fn map_types(
    &self,
    specifier: &ModuleSpecifier,
  ) -> Option<PackageMappedSpecifier> {
    if specifier.path().starts_with("/-/") {
      // ignore, it's an internal url whose version is a build hash
      return None;
    }

    let text = without_query(specifier);
    let captures = SKYPACK_MAPPING_RE.captures(&text)?;
    to_types_package(&captures)
  }
}

struct EsmShMapper;

impl SpecifierMapper for EsmShMapper {
  fn map(&self, specifier: &ModuleSpecifier) -> Option<PackageMappedSpecifier> {
    // Ignore esm.sh imports that are from a github repo. Perhaps in the
    // future this could use a git specifier.
    if specifier.path().starts_with("/gh/") {
      return None;
    }

    // Ignore esm.sh imports of jsr packages. These are only published to
    // npm under the `@jsr` scope on the jsr registry, which would require
    // the output package to be configured with a custom registry, so just
    // vendor the remote module instead.
    if specifier.path().starts_with("/jsr/") {
      return None;
    }

    let captures = ESMSH_MAPPING_RE.captures(specifier.as_str())?;

    let sub_path = captures.get(4).map(|m| m.as_str().to_owned());

    // don't use the package for declaration file imports
    if let Some(sub_path) = &sub_path {
      // todo(dsherret): this should probably work on media type
      if sub_path.to_lowercase().ends_with(".d.ts") {
        return None;
      }
    }

    Some(PackageMappedSpecifier {
      name: captures.get(2).unwrap().as_str().to_string(),
      version: Some(captures.get(3).unwrap().as_str().to_string()),
      sub_path,
      peer_dependency: false,
    })
  }

  fn map_types(
    &self,
    specifier: &ModuleSpecifier,
  ) -> Option<PackageMappedSpecifier> {
    if specifier.path().starts_with("/gh/")
      || specifier.path().starts_with("/jsr/")
    {
      return None;
    }

    let text = without_query(specifier);
    let captures = ESMSH_MAPPING_RE.captures(&text)?;
    to_types_package(&captures)
  }
}

/// Creates a package for a declaration file url captured by one of the
/// cdn regexes (ex. `https://esm.sh/@types/node@22.0.0/index.d.ts`).
fn to_types_package(
  captures: &regex::Captures,
) -> Option<PackageMappedSpecifier> {
  Some(PackageMappedSpecifier {
    name: captures.get(2)?.as_str().to_string(),
    version: Some(
      captures
        .get(3)?
        .as_str()
        .trim_start_matches('v')
        .to_string(),
    ),
    // the sub path is dropped because the package is only used as a
    // dependency in the package.json and never in a module specifier
    sub_path: None,
    peer_dependency: false,
  })
}

fn without_query(specifier: &ModuleSpecifier) -> String {
  let text = specifier.as_str();
  match text.find(['?', '#']) {
    Some(index) => text[..index].to_string(),
    None => text.to_string(),
  }
}

struct DenoStdNodeSpecifierMapper {
  url_re: Regex,
  to_specifier: String,
}

impl DenoStdNodeSpecifierMapper {
  pub fn new(package: impl AsRef<str>) -> Self {
    Self {
      url_re: Regex::new(&format!(
        r"^https://deno\.land/std(@[0-9]+\.[0-9]+\.[0-9]+)?/node/{}\.ts",
        package.as_ref()
      ))
      .unwrap(),
      to_specifier: package.as_ref().to_owned(),
    }
  }
}

impl SpecifierMapper for DenoStdNodeSpecifierMapper {
  fn map(&self, specifier: &ModuleSpecifier) -> Option<PackageMappedSpecifier> {
    if self.url_re.is_match(specifier.as_str()) {
      Some(PackageMappedSpecifier {
        name: self.to_specifier.clone(),
        version: None,
        sub_path: None,
        peer_dependency: false,
      })
    } else {
      None
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_skypack_mapper() {
    let mapper = SkypackMapper;
    assert_eq!(
      mapper.map(
        &ModuleSpecifier::parse("https://cdn.skypack.dev/@project/name")
          .unwrap()
      ),
      None,
    );
    assert_eq!(
      mapper.map(
        &ModuleSpecifier::parse("https://cdn.skypack.dev/@project/name@v5.6.2")
          .unwrap()
      ),
      Some(PackageMappedSpecifier {
        name: "@project/name".to_string(),
        version: Some("5.6.2".to_string()),
        peer_dependency: false,
        sub_path: None,
      }),
    );
    assert_eq!(
      mapper.map(&ModuleSpecifier::parse("https://cdn.skypack.dev/-/@project/name@v5.6.2-hbht5UfbVmWkq5PkNraB/mode=imports/optimized/@project/name.js").unwrap()),
      None,
    );
  }

  #[test]
  fn test_esm_sh_mapper() {
    let mapper = EsmShMapper;
    assert_eq!(
      mapper
        .map(&ModuleSpecifier::parse("https://esm.sh/@project/name").unwrap()),
      None,
    );
    assert_eq!(
      mapper.map(
        &ModuleSpecifier::parse("https://esm.sh/@project/name@5.6.2").unwrap()
      ),
      Some(PackageMappedSpecifier {
        name: "@project/name".to_string(),
        version: Some("5.6.2".to_string()),
        peer_dependency: false,
        sub_path: None,
      }),
    );
    assert_eq!(
      mapper.map(
        &ModuleSpecifier::parse(
          "https://esm.sh/v86/@project/name@5.6.2/es2022/name.js"
        )
        .unwrap()
      ),
      Some(PackageMappedSpecifier {
        name: "@project/name".to_string(),
        version: Some("5.6.2".to_string()),
        sub_path: Some("es2022/name.js".to_string()),
        peer_dependency: false
      }),
    );
    assert_eq!(
      mapper.map(
        &ModuleSpecifier::parse("https://esm.sh/v114/nostr-tools@1.8.4")
          .unwrap()
      ),
      Some(PackageMappedSpecifier {
        name: "nostr-tools".to_string(),
        version: Some("1.8.4".to_string()),
        peer_dependency: false,
        sub_path: None,
      }),
    );
    assert_eq!(
      mapper
        .map(&ModuleSpecifier::parse("https://esm.sh/gh/owner/repo").unwrap()),
      None,
    );
    assert_eq!(
      mapper.map(
        &ModuleSpecifier::parse("https://esm.sh/jsr/@cross/fs@0.1.11").unwrap()
      ),
      None,
    );
    assert_eq!(
      mapper.map(
        &ModuleSpecifier::parse(
          "https://esm.sh/jsr/@std/path@1.0.8/resolve.ts"
        )
        .unwrap()
      ),
      None,
    );
  }

  #[test]
  fn map_types_esm_sh() {
    assert_types_package(
      "https://esm.sh/@types/svg-path-parser@~1.1.6/index.d.ts",
      Some(("@types/svg-path-parser", "~1.1.6")),
    );
    // versioned cdn path
    assert_types_package(
      "https://esm.sh/v135/@types/react@18.2.0/index.d.ts",
      Some(("@types/react", "18.2.0")),
    );
    // nested sub paths and other declaration file extensions
    assert_types_package(
      "https://esm.sh/@types/node@22.0.0/ts5.6/index.d.mts",
      Some(("@types/node", "22.0.0")),
    );
    assert_types_package(
      "https://esm.sh/@types/node@22.0.0/index.d.cts",
      Some(("@types/node", "22.0.0")),
    );
    // no sub path
    assert_types_package(
      "https://esm.sh/@types/node@22.0.0",
      Some(("@types/node", "22.0.0")),
    );
    // query strings
    assert_types_package(
      "https://esm.sh/@types/node@22.0.0/index.d.ts?target=denonext",
      Some(("@types/node", "22.0.0")),
    );
    // github and jsr packages aren't on npm
    assert_types_package("https://esm.sh/gh/user/repo@1.0.0/index.d.ts", None);
    assert_types_package(
      "https://esm.sh/jsr/@scope/name@1.0.0/index.d.ts",
      None,
    );
  }

  #[test]
  fn map_types_skypack() {
    assert_types_package(
      "https://cdn.skypack.dev/@types/lodash@4.14.191/index.d.ts",
      Some(("@types/lodash", "4.14.191")),
    );
    // internal urls have a build hash for a version
    assert_types_package(
        "https://cdn.skypack.dev/-/@types/lodash@v4.14.191-abcXYZ/dist=es2019/index.d.ts",
        None,
      );
  }

  #[test]
  fn map_types_unknown_cdn() {
    assert_types_package("https://deno.land/x/mod@1.0.0/mod.d.ts", None);
  }

  fn assert_types_package(specifier: &str, expected: Option<(&str, &str)>) {
    let mappers = get_all_specifier_mappers();
    let specifier = ModuleSpecifier::parse(specifier).unwrap();
    let result = get_types_package_for_specifier(&mappers, &specifier);
    assert_eq!(
      result.map(|p| (p.name, p.version.unwrap())),
      expected.map(|(n, v)| (n.to_string(), v.to_string()))
    );
  }
}
