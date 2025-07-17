// Copyright 2018-2024 the Deno authors. MIT license.

mod utils;

use std::collections::HashMap;

use anyhow::Context;
use anyhow::Result;
use dnt::MappedSpecifier;
use dnt::ModuleSpecifier;
use dnt::ScriptTarget;
use dnt::Shim;
use serde::Deserialize;
use utils::set_panic_hook;

use deno_cache_dir::file_fetcher::HeaderMap;
use deno_cache_dir::file_fetcher::HeaderName;
use deno_cache_dir::file_fetcher::HeaderValue;
use deno_cache_dir::file_fetcher::SendError;
use deno_cache_dir::file_fetcher::SendResponse;
use deno_cache_dir::file_fetcher::StatusCode;
use js_sys::Array;
use js_sys::Object;
use js_sys::Reflect;
use url::Url;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

#[wasm_bindgen(module = "/helpers.js")]
extern "C" {
  async fn fetch_specifier(specifier: String, headers: JsValue) -> JsValue;
}

enum FetchResult {
  Response(Response),
  Error(FetchError),
}

#[derive(Deserialize)]
struct FetchError {
  pub error: String,
}

struct Response {
  pub status: u16,
  pub body: Vec<u8>,
  pub headers: HeaderMap,
}

async fn fetch_specifier_typed(
  specifier: &str,
  headers: Vec<(String, String)>,
) -> Result<FetchResult, anyhow::Error> {
  let headers = headers_to_js_object(&headers);
  let response = fetch_specifier(specifier.to_string(), headers).await;
  parse_fetch_result(response).map_err(|err| {
    if let Some(s) = err.as_string() {
      anyhow::anyhow!(s)
    } else {
      // Optionally stringify complex JS error objects
      anyhow::anyhow!(format!("{:?}", err))
    }
  })
}

#[derive(Debug, Default, Clone)]
pub struct WasmHttpClient {
  pub cached_only: bool,
}

#[async_trait::async_trait(?Send)]
impl deno_cache_dir::file_fetcher::HttpClient for WasmHttpClient {
  async fn send_no_follow(
    &self,
    url: &Url,
    headers: HeaderMap,
  ) -> Result<SendResponse, SendError> {
    if self.cached_only {
      return Err(SendError::Failed(Box::new(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Cannot download because --cached-only was specified.",
      ))));
    }
    let headers = headers
      .into_iter()
      .filter_map(|(k, v)| Some((k?.to_string(), v.to_str().ok()?.to_string())))
      .collect::<Vec<(String, String)>>();
    let result =
      fetch_specifier_typed(url.as_str(), headers)
        .await
        .map_err(|err| {
          SendError::Failed(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            err.to_string(),
          )))
        })?;
    let response = match result {
      FetchResult::Response(response) => response,
      FetchResult::Error(fetch_error) => {
        return Err(SendError::Failed(fetch_error.error.into()));
      }
    };
    match response.status {
      304 => Ok(SendResponse::NotModified),
      300..=399 => Ok(SendResponse::Redirect(response.headers)),
      404 => Err(SendError::NotFound),
      200..=299 => Ok(SendResponse::Success(response.headers, response.body)),
      _ => Err(SendError::StatusCode(
        StatusCode::from_u16(response.status).unwrap(),
      )),
    }
  }
}

fn headers_to_js_object(headers: &[(String, String)]) -> JsValue {
  let obj = Object::new();
  for (key, value) in headers {
    Reflect::set(&obj, &JsValue::from_str(key), &JsValue::from_str(value))
      .unwrap();
  }
  obj.into()
}

fn parse_fetch_result(js_value: JsValue) -> Result<FetchResult, JsValue> {
  let has_error = Reflect::has(&js_value, &JsValue::from_str("error"))?;
  if has_error {
    let error: FetchError = serde_wasm_bindgen::from_value(js_value)?;
    return Ok(FetchResult::Error(error));
  }
  Ok(FetchResult::Response(parse_response(js_value)?))
}

fn parse_response(js_value: JsValue) -> Result<Response, JsValue> {
  let status = Reflect::get(&js_value, &JsValue::from_str("status"))?
    .as_f64()
    .ok_or_else(|| JsValue::from_str("status must be a number"))?
    as u16;

  let body_js = Reflect::get(&js_value, &JsValue::from_str("body"))?;
  let body: Vec<u8> = serde_wasm_bindgen::from_value(body_js)?;

  let headers_js = Reflect::get(&js_value, &JsValue::from_str("headers"))?;
  let headers = response_headers_to_headermap(headers_js);

  Ok(Response {
    status,
    body,
    headers,
  })
}

fn response_headers_to_headermap(headers: JsValue) -> HeaderMap {
  let mut map = HeaderMap::new();
  let entries_fn = Reflect::get(&headers, &JsValue::from_str("entries"));
  let Ok(entries_fn) = entries_fn else {
    return map;
  };

  let entries_iter = js_sys::Function::from(entries_fn)
    .call0(&headers)
    .ok()
    .and_then(|iter| iter.dyn_into::<js_sys::Iterator>().ok());

  let Some(iter) = entries_iter else {
    return map;
  };

  while let Ok(next) = iter.next() {
    if next.done() {
      break;
    }

    let val = next.value();
    let pair = Array::from(&val);
    if pair.length() != 2 {
      continue;
    }

    let key = pair.get(0).as_string();
    let value = pair.get(1).as_string();

    if let (Some(k), Some(v)) = (key, value) {
      if let (Ok(name), Ok(val)) = (
        HeaderName::from_bytes(k.as_bytes()),
        HeaderValue::from_str(&v),
      ) {
        map.append(name, val);
      }
    }
  }

  map
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
  pub config_file: Option<ModuleSpecifier>,
  pub cwd: ModuleSpecifier,
}

#[wasm_bindgen]
pub async fn transform(options: JsValue) -> Result<JsValue, JsValue> {
  set_panic_hook();

  transform_inner(options)
    .await
    // need to include the anyhow context
    .map_err(|err| format!("{:#}", err).into())
}

async fn transform_inner(options: JsValue) -> Result<JsValue, anyhow::Error> {
  #[allow(deprecated)]
  let options: TransformOptions = options.into_serde()?;
  // todo(dsherret): try using this again sometime in the future... it errored
  // with "invalid type: unit value, expected a boolean" and didn't say exactly
  // where it errored.
  // let options: TransformOptions = serde_wasm_bindgen::from_value(options)?;

  let result = dnt::transform(
    sys_traits::impls::RealSys,
    WasmHttpClient { cached_only: false },
    dnt::TransformOptions {
      entry_points: parse_module_specifiers(options.entry_points)?,
      test_entry_points: parse_module_specifiers(options.test_entry_points)?,
      shims: options.shims,
      test_shims: options.test_shims,
      specifier_mappings: options.mappings,
      target: options.target,
      import_map: options.import_map,
      config_file: options.config_file,
      cwd: deno_path_util::url_to_file_path(&options.cwd)?,
    },
  )
  .await?;
  Ok(serde_wasm_bindgen::to_value(&result).unwrap())
}

fn parse_module_specifiers(
  values: Vec<String>,
) -> Result<Vec<ModuleSpecifier>, anyhow::Error> {
  let mut specifiers = Vec::with_capacity(values.len());
  for value in values {
    specifiers.push(parse_module_specifier(&value)?);
  }
  Ok(specifiers)
}

fn parse_module_specifier(
  value: &str,
) -> Result<ModuleSpecifier, anyhow::Error> {
  ModuleSpecifier::parse(value)
    .with_context(|| format!("Error parsing {}.", value))
}
