use anyhow::Result;
use deno_node_transform::transform;
use deno_node_transform::ModuleSpecifier;
use deno_node_transform::TransformOptions;
use deno_node_transform::TransformOutput;

use super::InMemoryLoader;

pub struct TestBuilder {
  loader: InMemoryLoader,
  entry_point: String,
  shim_package_name: Option<String>,
}

impl TestBuilder {
  pub fn new() -> Self {
    let loader = InMemoryLoader::new();
    Self {
      loader,
      entry_point: "file:///mod.ts".to_string(),
      shim_package_name: None,
    }
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

  pub fn shim_package_name(&mut self, name: impl AsRef<str>) -> &mut Self {
    self.shim_package_name = Some(name.as_ref().to_string());
    self
  }

  pub async fn transform(&self) -> Result<TransformOutput> {
    transform(TransformOptions {
      entry_point: ModuleSpecifier::parse(&self.entry_point).unwrap(),
      shim_package_name: self
        .shim_package_name
        .as_ref()
        .map(ToOwned::to_owned)
        .unwrap_or("deno.ns".to_string()),
      loader: Some(Box::new(self.loader.clone())),
    })
    .await
  }
}
