// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

mod utils;

use std::collections::HashMap;
use std::future::Future;

use anyhow::Result;
use dnt::MappedSpecifier;
use dnt::ModuleSpecifier;
use dnt::ScriptTarget;
use dnt::Shim;
use serde::Deserialize;
use utils::set_panic_hook;
use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen(module = "/helpers.js")]
extern "C" {
  async fn fetch_specifier(specifier: String) -> JsValue;
}

struct JsLoader {}

impl dnt::Loader for JsLoader {
  fn load(
    &self,
    url: dnt::ModuleSpecifier,
  ) -> std::pin::Pin<
    Box<dyn Future<Output = Result<Option<dnt::LoadResponse>>> + 'static>,
  > {
    Box::pin(async move {
      let resp = fetch_specifier(url.to_string()).await;
      if resp.is_null() || resp.is_undefined() {
        return Ok(None);
      }
      if !resp.is_object() {
        anyhow::bail!("fetch response wasn't an object");
      }
      let load_response = serde_wasm_bindgen::from_value(resp).unwrap();
      Ok(Some(load_response))
    })
  }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformOptions {
  pub entry_points: Vec<String>,
  pub test_entry_points: Vec<String>,
  pub shims: Vec<Shim>,
  pub test_shims: Vec<Shim>,
  pub mappings: HashMap<ModuleSpecifier, MappedSpecifier>,
  pub target: ScriptTarget,
  pub import_map: Option<ModuleSpecifier>,
}

#[wasm_bindgen]
pub async fn transform(options: JsValue) -> Result<JsValue, JsValue> {
  set_panic_hook();

  let options: TransformOptions =
    serde_wasm_bindgen::from_value(options).unwrap();
  let result = dnt::transform(dnt::TransformOptions {
    entry_points: parse_module_specifiers(options.entry_points)?,
    test_entry_points: parse_module_specifiers(options.test_entry_points)?,
    shims: options.shims,
    test_shims: options.test_shims,
    loader: Some(Box::new(JsLoader {})),
    specifier_mappings: options.mappings,
    target: options.target,
    import_map: options.import_map,
  })
  .await
  .map_err(|err| format!("{:?}", err))?; // need to include the anyhow context

  Ok(serde_wasm_bindgen::to_value(&result).unwrap())
}

fn parse_module_specifiers(
  values: Vec<String>,
) -> Result<Vec<ModuleSpecifier>, JsValue> {
  let mut specifiers = Vec::new();
  for value in values {
    let entry_point = dnt::ModuleSpecifier::parse(&value)
      .map_err(|err| format!("Error parsing {}. {}", value, err))?;
    specifiers.push(entry_point);
  }
  Ok(specifiers)
}
