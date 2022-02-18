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
    Box::new(NodeSpecifierMapper::new("buffer")),
    Box::new(NodeSpecifierMapper::new("child_process")),
    Box::new(NodeSpecifierMapper::new("console")),
    Box::new(NodeSpecifierMapper::new("constants")),
    Box::new(NodeSpecifierMapper::new("crypto")),
    Box::new(NodeSpecifierMapper::new("events")),
    Box::new(NodeSpecifierMapper::new("fs")),
    Box::new(NodeSpecifierMapper::new("fs/promises")),
    Box::new(NodeSpecifierMapper::new("module")),
    Box::new(NodeSpecifierMapper::new("os")),
    Box::new(NodeSpecifierMapper::new("path")),
    Box::new(NodeSpecifierMapper::new("process")),
    Box::new(NodeSpecifierMapper::new("querystring")),
    Box::new(NodeSpecifierMapper::new("stream")),
    Box::new(NodeSpecifierMapper::new("string_decoder")),
    Box::new(NodeSpecifierMapper::new("timers")),
    Box::new(NodeSpecifierMapper::new("tty")),
    Box::new(NodeSpecifierMapper::new("url")),
    Box::new(NodeSpecifierMapper::new("util")),
    Box::new(SkypackMapper {}),
    Box::new(EsmShMapper {}),
  ]
}

// good enough for a first pass
static SKYPACK_MAPPING_RE: Lazy<Regex> = Lazy::new(|| {
  Regex::new(
    r"^https://cdn\.skypack\.dev/(@?[^@?]+)@([0-9.\^~\-A-Za-z]+)(?:/([^#?]+))?",
  )
  .unwrap()
});
static ESMSH_MAPPING_RE: Lazy<Regex> = Lazy::new(|| {
  Regex::new(r"^https://esm\.sh/(@?[^@?]+)@([0-9.\^~\-A-Za-z]+)(?:/([^#?]+))?$")
    .unwrap()
});

struct SkypackMapper {}

impl SpecifierMapper for SkypackMapper {
  fn map(&self, specifier: &ModuleSpecifier) -> Option<PackageMappedSpecifier> {
    SKYPACK_MAPPING_RE
      .captures(specifier.as_str())
      .map(|captures| PackageMappedSpecifier {
        name: captures.get(1).unwrap().as_str().to_string(),
        version: Some(captures.get(2).unwrap().as_str().to_string()),
        sub_path: captures.get(3).map(|m| m.as_str().to_owned()),
      })
  }
}

struct EsmShMapper {}

impl SpecifierMapper for EsmShMapper {
  fn map(&self, specifier: &ModuleSpecifier) -> Option<PackageMappedSpecifier> {
    ESMSH_MAPPING_RE
      .captures(specifier.as_str())
      .map(|captures| PackageMappedSpecifier {
        name: captures.get(1).unwrap().as_str().to_string(),
        version: Some(captures.get(2).unwrap().as_str().to_string()),
        sub_path: captures.get(3).map(|m| m.as_str().to_owned()),
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
      })
    } else {
      None
    }
  }
}
