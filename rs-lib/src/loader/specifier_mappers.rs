// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use deno_ast::ModuleSpecifier;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::PackageMappedSpecifier;

pub trait SpecifierMapper {
  fn map(&self, specifier: &ModuleSpecifier) -> Option<PackageMappedSpecifier>;
}

pub fn get_all_specifier_mappers() -> Vec<Box<dyn SpecifierMapper>> {
  vec![
    Box::new(NodeSpecifierMapper::new("assert")),
    Box::new(NodeSpecifierMapper::new("assert/strict")),
    Box::new(NodeSpecifierMapper::new("buffer")),
    Box::new(NodeSpecifierMapper::new("console")),
    Box::new(NodeSpecifierMapper::new("constants")),
    Box::new(NodeSpecifierMapper::new("crypto")),
    Box::new(NodeSpecifierMapper::new("child_process")),
    Box::new(NodeSpecifierMapper::new("dns")),
    Box::new(NodeSpecifierMapper::new("events")),
    Box::new(NodeSpecifierMapper::new("fs")),
    Box::new(NodeSpecifierMapper::new("fs/promises")),
    Box::new(NodeSpecifierMapper::new("http")),
    Box::new(NodeSpecifierMapper::new("module")),
    Box::new(NodeSpecifierMapper::new("net")),
    Box::new(NodeSpecifierMapper::new("os")),
    Box::new(NodeSpecifierMapper::new("path")),
    Box::new(NodeSpecifierMapper::new("perf_hooks")),
    Box::new(NodeSpecifierMapper::new("process")),
    Box::new(NodeSpecifierMapper::new("querystring")),
    Box::new(NodeSpecifierMapper::new("readline")),
    Box::new(NodeSpecifierMapper::new("stream")),
    Box::new(NodeSpecifierMapper::new("string_decoder")),
    Box::new(NodeSpecifierMapper::new("sys")),
    Box::new(NodeSpecifierMapper::new("timers")),
    Box::new(NodeSpecifierMapper::new("timers/promises")),
    Box::new(NodeSpecifierMapper::new("tty")),
    Box::new(NodeSpecifierMapper::new("url")),
    Box::new(NodeSpecifierMapper::new("util")),
    Box::new(NodeSpecifierMapper::new("worker_threads")),
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
  Regex::new(r"^https://esm\.sh/(@?[^@?]+)@([0-9.\^~\-A-Za-z]+)(?:/([^#?]+))?$")
    .unwrap()
});
static ESMSH_IGNORE_MAPPING_RE: Lazy<Regex> = Lazy::new(|| {
  // internal urls
  Regex::new(r"^https://esm\.sh/v[0-9]+/.*/.*/").unwrap()
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
}

struct EsmShMapper;

impl SpecifierMapper for EsmShMapper {
  fn map(&self, specifier: &ModuleSpecifier) -> Option<PackageMappedSpecifier> {
    let captures = ESMSH_MAPPING_RE.captures(specifier.as_str())?;

    if ESMSH_IGNORE_MAPPING_RE.is_match(specifier.as_str()) {
      // ignore, as it's internal
      return None;
    }

    let sub_path = captures.get(3).map(|m| m.as_str().to_owned());

    // don't use the package for declaration file imports
    if let Some(sub_path) = &sub_path {
      // todo(dsherret): this should probably work on media type
      if sub_path.to_lowercase().ends_with(".d.ts") {
        return None;
      }
    }

    Some(PackageMappedSpecifier {
      name: captures.get(1).unwrap().as_str().to_string(),
      version: Some(captures.get(2).unwrap().as_str().to_string()),
      sub_path,
      peer_dependency: false,
    })
  }
}

struct NodeSpecifierMapper {
  url_re: Regex,
  to_specifier: String,
}

impl NodeSpecifierMapper {
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

impl SpecifierMapper for NodeSpecifierMapper {
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
      None,
    );
  }
}
