// Copyright 2018-2024 the Deno authors. MIT license.

use std::collections::HashMap;
use std::rc::Rc;

use anyhow::Result;
use deno_node_transform::transform;
use deno_node_transform::GlobalName;
use deno_node_transform::MappedSpecifier;
use deno_node_transform::ModuleSpecifier;
use deno_node_transform::PackageMappedSpecifier;
use deno_node_transform::PackageShim;
use deno_node_transform::ScriptTarget;
use deno_node_transform::Shim;
use deno_node_transform::TransformOptions;
use deno_node_transform::TransformOutput;
use sys_traits::impls::InMemorySys;
use sys_traits::EnvCurrentDir;

use super::InMemoryLoader;

pub struct TestBuilder {
  loader: InMemoryLoader,
  entry_point: String,
  additional_entry_points: Vec<String>,
  test_entry_points: Vec<String>,
  specifier_mappings: HashMap<ModuleSpecifier, MappedSpecifier>,
  shims: Vec<Shim>,
  test_shims: Vec<Shim>,
  target: ScriptTarget,
  config_file: Option<ModuleSpecifier>,
  import_map: Option<ModuleSpecifier>,
}

impl TestBuilder {
  pub fn new() -> Self {
    Self {
      loader: InMemoryLoader::new(),
      entry_point: "file:///mod.ts".to_string(),
      additional_entry_points: Vec::new(),
      test_entry_points: Vec::new(),
      specifier_mappings: Default::default(),
      shims: Default::default(),
      test_shims: Default::default(),
      target: ScriptTarget::ES5,
      config_file: None,
      import_map: None,
    }
  }

  pub fn with_sys(
    &mut self,
    mut action: impl FnMut(&mut InMemorySys),
  ) -> &mut Self {
    action(&mut self.loader.sys);
    self
  }

  pub fn with_loader(
    &mut self,
    mut action: impl FnMut(&mut InMemoryLoader),
  ) -> &mut Self {
    action(&mut self.loader);
    self
  }

  pub fn entry_point(&mut self, value: impl AsRef<str>) -> &mut Self {
    self.entry_point = value.as_ref().to_string();
    self
  }

  pub fn add_entry_point(&mut self, value: impl AsRef<str>) -> &mut Self {
    self
      .additional_entry_points
      .push(value.as_ref().to_string());
    self
  }

  pub fn add_test_entry_point(&mut self, value: impl AsRef<str>) -> &mut Self {
    self.test_entry_points.push(value.as_ref().to_string());
    self
  }

  pub fn set_config_file(&mut self, url: impl AsRef<str>) -> &mut Self {
    self.import_map = Some(ModuleSpecifier::parse(url.as_ref()).unwrap());
    self
  }

  pub fn set_import_map(&mut self, url: impl AsRef<str>) -> &mut Self {
    self.import_map = Some(ModuleSpecifier::parse(url.as_ref()).unwrap());
    self
  }

  pub fn add_default_shims(&mut self) -> &mut Self {
    let deno_shim = Shim::Package(PackageShim {
      package: PackageMappedSpecifier {
        name: "@deno/shim-deno".to_string(),
        version: Some("^0.1.0".to_string()),
        sub_path: None,
        peer_dependency: false,
      },
      types_package: None,
      global_names: vec![GlobalName {
        name: "Deno".to_string(),
        export_name: None,
        type_only: false,
      }],
    });
    self.add_shim(deno_shim.clone());
    self.add_test_shim(deno_shim);
    let timers_shim = Shim::Package(PackageShim {
      package: PackageMappedSpecifier {
        name: "@deno/shim-timers".to_string(),
        version: Some("^0.1.0".to_string()),
        sub_path: None,
        peer_dependency: false,
      },
      types_package: None,
      global_names: vec![
        GlobalName {
          name: "setTimeout".to_string(),
          export_name: None,
          type_only: false,
        },
        GlobalName {
          name: "setInterval".to_string(),
          export_name: None,
          type_only: false,
        },
      ],
    });
    self.add_shim(timers_shim.clone());
    self.add_test_shim(timers_shim);
    self
  }

  pub fn add_shim(&mut self, shim: Shim) -> &mut Self {
    self.shims.push(shim);
    self
  }

  pub fn add_test_shim(&mut self, shim: Shim) -> &mut Self {
    self.test_shims.push(shim);
    self
  }

  pub fn add_package_specifier_mapping(
    &mut self,
    specifier: impl AsRef<str>,
    bare_specifier: impl AsRef<str>,
    version: Option<&str>,
    path: Option<&str>,
  ) -> &mut Self {
    self.specifier_mappings.insert(
      ModuleSpecifier::parse(specifier.as_ref()).unwrap(),
      MappedSpecifier::Package(PackageMappedSpecifier {
        name: bare_specifier.as_ref().to_string(),
        version: version.map(|v| v.to_string()),
        sub_path: path.map(|v| v.to_string()),
        peer_dependency: false,
      }),
    );
    self
  }

  pub fn add_module_specifier_mapping(
    &mut self,
    from: impl AsRef<str>,
    to: impl AsRef<str>,
  ) -> &mut Self {
    self.specifier_mappings.insert(
      ModuleSpecifier::parse(from.as_ref()).unwrap(),
      MappedSpecifier::Module(ModuleSpecifier::parse(to.as_ref()).unwrap()),
    );
    self
  }

  pub fn set_target(&mut self, target: ScriptTarget) -> &mut Self {
    self.target = target;
    self
  }

  pub async fn transform(&self) -> Result<TransformOutput> {
    let mut entry_points =
      vec![ModuleSpecifier::parse(&self.entry_point).unwrap()];
    entry_points.extend(
      self
        .additional_entry_points
        .iter()
        .map(|p| ModuleSpecifier::parse(p).unwrap()),
    );
    transform(
      self.loader.sys.clone(),
      TransformOptions {
        entry_points,
        test_entry_points: self
          .test_entry_points
          .iter()
          .map(|p| ModuleSpecifier::parse(p).unwrap())
          .collect(),
        shims: self.shims.clone(),
        test_shims: self.test_shims.clone(),
        loader: Some(Rc::new(self.loader.clone())),
        specifier_mappings: self.specifier_mappings.clone(),
        target: self.target,
        config_file: self.config_file.clone(),
        import_map: self.import_map.clone(),
        cwd: self.loader.sys.env_current_dir().unwrap(),
      },
    )
    .await
  }
}
