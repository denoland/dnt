// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

mod utils;

use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;

use anyhow::Result;
use dnt::ModuleSpecifier;
use serde::Deserialize;
use utils::set_panic_hook;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen(module = "/helpers.js")]
extern "C" {
  fn read_file_sync(file_path: String) -> String;
}

struct JsLoader {}

impl dnt::Loader for JsLoader {
  fn read_file(
    &self,
    file_path: PathBuf,
  ) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = std::io::Result<String>> + 'static>,
  > {
    Box::pin(async move {
      Ok(read_file_sync(file_path.to_string_lossy().to_string()))
    })
  }

  fn make_request(
    &self,
    url: dnt::ModuleSpecifier,
  ) -> std::pin::Pin<
    Box<dyn Future<Output = Result<dnt::LoadResponse>> + 'static>,
  > {
    Box::pin(async move {
      // todo: handle error
      let mut opts = RequestInit::new();
      opts.method("GET");
      opts.mode(RequestMode::Cors);
      let request =
        Request::new_with_str_and_init(&url.to_string(), &opts).unwrap();
      let window = web_sys::window().unwrap();
      let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .unwrap();
      assert!(resp_value.is_instance_of::<Response>());
      let resp: Response = resp_value.dyn_into().unwrap();
      let text = JsFuture::from(resp.text().unwrap()).await.unwrap();
      Ok(dnt::LoadResponse {
        content: text.as_string().unwrap(),
        maybe_headers: None,
      })
    })
  }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformOptions {
  pub entry_point: String,
  pub shim_package_name: String,
  pub specifier_mappings: Option<HashMap<ModuleSpecifier, String>>,
}

#[wasm_bindgen]
pub async fn transform(options: JsValue) -> Result<JsValue, JsValue> {
  set_panic_hook();

  let options: TransformOptions = options.into_serde().unwrap();

  let result = dnt::transform(dnt::TransformOptions {
    entry_point: dnt::ModuleSpecifier::parse(&options.entry_point).unwrap(),
    shim_package_name: options.shim_package_name,
    loader: Some(Box::new(JsLoader {})),
    specifier_mappings: options.specifier_mappings,
  })
  .await
  .unwrap();

  Ok(JsValue::from_serde(&result).unwrap())
}
