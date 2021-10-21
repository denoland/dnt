// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use deno_ast::ModuleSpecifier;
use regex::Regex;

use crate::MappedSpecifier;

pub trait SpecifierMapper {
  fn map(&self, specifier: &ModuleSpecifier) -> Option<MappedSpecifier>;
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

lazy_static! {
  // good enough for a first pass
  static ref SKYPACK_MAPPING_RE: Regex = Regex::new(r"^https://cdn\.skypack\.dev/(@?[^@?]+)@([0-9\.\^~\-A-Za-z]+)").unwrap();
  static ref ESMSH_MAPPING_RE: Regex = Regex::new(r"^https://esm\.sh/(@?[^@?]+)@([0-9\.\^~\-A-Za-z]+)$").unwrap();
}

struct SkypackMapper {}

impl SpecifierMapper for SkypackMapper {
  fn map(&self, specifier: &ModuleSpecifier) -> Option<MappedSpecifier> {
    SKYPACK_MAPPING_RE
      .captures(specifier.as_str())
      .map(|captures| MappedSpecifier {
        name: captures.get(1).unwrap().as_str().to_string(),
        version: Some(captures.get(2).unwrap().as_str().to_string()),
      })
  }
}

struct EsmShMapper {}

impl SpecifierMapper for EsmShMapper {
  fn map(&self, specifier: &ModuleSpecifier) -> Option<MappedSpecifier> {
    ESMSH_MAPPING_RE
      .captures(specifier.as_str())
      .map(|captures| MappedSpecifier {
        name: captures.get(1).unwrap().as_str().to_string(),
        version: Some(captures.get(2).unwrap().as_str().to_string()),
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
  fn map(&self, specifier: &ModuleSpecifier) -> Option<MappedSpecifier> {
    if self.url_re.is_match(specifier.as_str()) {
      Some(MappedSpecifier {
        name: self.to_specifier.clone(),
        version: None,
      })
    } else {
      None
    }
  }
}
