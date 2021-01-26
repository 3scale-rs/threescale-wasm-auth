use anyhow::{anyhow, Result};
use straitjacket::resources::http::endpoint::Endpoint;

pub struct SystemClient {
    pub url: url::Url,
    pub cluster: String,
    pub token: String,
}

fn get_latest_proxy(svc_id: &str) {
    let str = straitjacket::api::v0::service::proxy::configs::LIST
        .path(&[svc_id, "latest"])
        .unwrap();
}

pub fn endpoint_to_path<T>(
    ep: &Endpoint<'_, '_, T>,
    args: &[&str],
    base_path: Option<&str>,
) -> Result<String> {
    let mut s = ep
        .path(args)
        .map_err(|e| anyhow!("could not build path for endpoint {}", e))?;
    if let Some(prefix) = base_path {
        s.insert_str(0, prefix)
    }

    Ok(s)
}

pub fn endpoint_call<C: proxy_wasm::traits::Context, T>(
    ctx: &C,
    cluster: &str,
    authority: &str,
    ep: &Endpoint<'_, '_, T>,
    args: &[&str],
    base_path: Option<&str>,
    body: Option<&[u8]>,
    timeout: core::time::Duration,
) -> Result<u32, anyhow::Error> {
    let path = endpoint_to_path(ep, args, base_path)?;
    let headers = vec![(":path", path.as_str()), (":authority", authority)];
    ctx.dispatch_http_call(cluster, headers, body, vec![], timeout)
        .map_err(|e| anyhow!("failed to dispatch HTTP call: {:?}", e))
}
