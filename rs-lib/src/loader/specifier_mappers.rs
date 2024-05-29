// Copyright 2018-2024 the Deno authors. MIT license.

use deno_ast::ModuleSpecifier;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::PackageMappedSpecifier;

pub trait SpecifierMapper {
  fn map(&self, specifier: &ModuleSpecifier) -> Option<PackageMappedSpecifier>;
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
    Box::new(JsrMapper),
    Box::new(SkypackMapper),
    Box::new(EsmShMapper),
    Box::new(NpmMapper),
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
}

struct EsmShMapper;

impl SpecifierMapper for EsmShMapper {
  fn map(&self, specifier: &ModuleSpecifier) -> Option<PackageMappedSpecifier> {
    // Ignore esm.sh imports that are from a github repo. Perhaps in the
    // future this could use a git specifier.
    if specifier.path().starts_with("/gh/") {
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
}

struct NpmMapper;

impl SpecifierMapper for NpmMapper {
  fn map(&self, specifier: &ModuleSpecifier) -> Option<PackageMappedSpecifier> {
    let npm_specifier =
      deno_semver::npm::NpmPackageReqReference::from_str(specifier.as_str())
        .ok()?;
    Some(PackageMappedSpecifier {
      name: npm_specifier.req().name.clone(),
      version: Some(npm_specifier.req().version_req.version_text().to_string()),
      sub_path: npm_specifier.sub_path().map(|s| s.to_string()),
      peer_dependency: false,
    })
  }
}

struct JsrMapper;

impl SpecifierMapper for JsrMapper {
  fn map(&self, specifier: &ModuleSpecifier) -> Option<PackageMappedSpecifier> {
    let req_ref =
      deno_semver::jsr::JsrPackageReqReference::from_specifier(specifier)
        .ok()?;
    Some(PackageMappedSpecifier {
      name: req_ref.req().name.clone(),
      version: Some(req_ref.req().version_req.to_string()),
      sub_path: req_ref.sub_path().map(|s| s.to_string()),
      peer_dependency: false,
    })
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
  }

  #[test]
  fn test_npm_mapper() {
    let mapper = NpmMapper;
    assert_eq!(
      mapper.map(&ModuleSpecifier::parse("npm:package").unwrap()),
      Some(PackageMappedSpecifier {
        name: "package".to_string(),
        version: Some("*".to_string()),
        sub_path: None,
        peer_dependency: false
      })
    );
    assert_eq!(
      mapper.map(&ModuleSpecifier::parse("npm:package@^2.1").unwrap()),
      Some(PackageMappedSpecifier {
        name: "package".to_string(),
        version: Some("^2.1".to_string()),
        sub_path: None,
        peer_dependency: false
      })
    );
    assert_eq!(
      mapper.map(&ModuleSpecifier::parse("npm:preact/hooks").unwrap()),
      Some(PackageMappedSpecifier {
        name: "preact".to_string(),
        version: Some("*".to_string()),
        sub_path: Some("hooks".to_string()),
        peer_dependency: false
      })
    );
    assert_eq!(
      mapper.map(&ModuleSpecifier::parse("npm:package/sub/path").unwrap()),
      Some(PackageMappedSpecifier {
        name: "package".to_string(),
        version: Some("*".to_string()),
        sub_path: Some("sub/path".to_string()),
        peer_dependency: false
      })
    );
    assert_eq!(
      mapper.map(&ModuleSpecifier::parse("npm:@scope/name/path/sub").unwrap()),
      Some(PackageMappedSpecifier {
        name: "@scope/name".to_string(),
        version: Some("*".to_string()),
        sub_path: Some("path/sub".to_string()),
        peer_dependency: false
      })
    );
    assert_eq!(
      mapper.map(&ModuleSpecifier::parse("npm:package@^2.1/sub_path").unwrap()),
      Some(PackageMappedSpecifier {
        name: "package".to_string(),
        version: Some("^2.1".to_string()),
        sub_path: Some("sub_path".to_string()),
        peer_dependency: false
      })
    );
    assert_eq!(
      mapper.map(&ModuleSpecifier::parse("npm:@project/name@2.1.3").unwrap()),
      Some(PackageMappedSpecifier {
        name: "@project/name".to_string(),
        version: Some("2.1.3".to_string()),
        sub_path: None,
        peer_dependency: false
      })
    );
    assert_eq!(
      mapper.map(&ModuleSpecifier::parse("npm:/@project/name@2.1.3").unwrap()),
      Some(PackageMappedSpecifier {
        name: "@project/name".to_string(),
        version: Some("2.1.3".to_string()),
        sub_path: None,
        peer_dependency: false
      })
    );
  }

  #[test]
  fn test_jsr_mapper() {
    let mapper = JsrMapper;
    assert_eq!(
      mapper.map(&ModuleSpecifier::parse("jsr:@scope/name").unwrap()),
      Some(PackageMappedSpecifier {
        name: "@scope/name".to_string(),
        version: Some("*".to_string()),
        sub_path: None,
        peer_dependency: false
      })
    );
    assert_eq!(
      mapper.map(&ModuleSpecifier::parse("jsr:name@2.1.3/mod.ts").unwrap()),
      Some(PackageMappedSpecifier {
        name: "name".to_string(),
        version: Some("2.1.3".to_string()),
        sub_path: Some("mod.ts".to_string()),
        peer_dependency: false
      })
    );
    assert_eq!(
      mapper.map(
        &ModuleSpecifier::parse("jsr:@project/name@2.1.3/mod.ts").unwrap()
      ),
      Some(PackageMappedSpecifier {
        name: "@project/name".to_string(),
        version: Some("2.1.3".to_string()),
        sub_path: Some("mod.ts".to_string()),
        peer_dependency: false
      })
    );
    assert_eq!(
      mapper.map(&ModuleSpecifier::parse("jsr:@project/name@2.1.3").unwrap()),
      Some(PackageMappedSpecifier {
        name: "@project/name".to_string(),
        version: Some("2.1.3".to_string()),
        sub_path: None,
        peer_dependency: false
      })
    );
    assert_eq!(
      mapper
        .map(&ModuleSpecifier::parse("jsr:@project/name/sub-path").unwrap()),
      Some(PackageMappedSpecifier {
        name: "@project/name".to_string(),
        version: Some("*".to_string()),
        sub_path: Some("sub-path".to_string()),
        peer_dependency: false
      })
    );
  }
}
