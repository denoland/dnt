// Copyright 2018-2024 the Deno authors. MIT license.

mod utils;

use std::collections::HashMap;
use std::future::Future;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::Result;
use deno_error::JsErrorBox;
use dnt::LoadError;
use dnt::MappedSpecifier;
use dnt::ModuleSpecifier;
use dnt::ScriptTarget;
use dnt::Shim;
use serde::Deserialize;
use utils::set_panic_hook;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/helpers.js")]
extern "C" {
  async fn fetch_specifier(
    specifier: String,
    cache_setting: u8,
    maybe_checksum: Option<String>,
  ) -> JsValue;
}

struct JsLoader;

impl dnt::Loader for JsLoader {
  fn load(
    &self,
    url: dnt::ModuleSpecifier,
    cache_setting: dnt::CacheSetting,
    maybe_checksum: Option<dnt::LoaderChecksum>,
  ) -> std::pin::Pin<
    Box<
      dyn Future<Output = Result<Option<dnt::LoadResponse>, LoadError>>
        + 'static,
    >,
  > {
    Box::pin(async move {
      let resp = fetch_specifier(
        url.to_string(),
        // WARNING: Ensure this matches wasm/helpers.js
        match cache_setting {
          dnt::CacheSetting::Only => 0,
          dnt::CacheSetting::Use => 1,
          dnt::CacheSetting::Reload => 2,
        },
        maybe_checksum.map(|c| c.into_string()),
      )
      .await;
      if resp.is_null() || resp.is_undefined() {
        return Ok(None);
      }
      if !resp.is_object() {
        return Err(LoadError::Other(Arc::new(JsErrorBox::generic(
          "fetch response wasn't an object",
        ))));
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

  #[allow(deprecated)]
  let options: TransformOptions = options.into_serde().unwrap();
  // todo(dsherret): try using this again sometime in the future... it errored
  // with "invalid type: unit value, expected a boolean" and didn't say exactly
  // where it errored.
  // let options: TransformOptions = serde_wasm_bindgen::from_value(options)?;

  let result = dnt::transform(dnt::TransformOptions {
    entry_points: parse_module_specifiers(options.entry_points)?,
    test_entry_points: parse_module_specifiers(options.test_entry_points)?,
    shims: options.shims,
    test_shims: options.test_shims,
    loader: Some(Rc::new(JsLoader {})),
    specifier_mappings: options.mappings,
    target: options.target,
    import_map: options.import_map,
  })
  .await
  .map_err(|err| format!("{:#}", err))?; // need to include the anyhow context

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
