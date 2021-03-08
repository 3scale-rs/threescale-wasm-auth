use std::vec;

use super::decode::Value;
use super::request_headers::RequestHeaders;
use super::HttpAuthThreescale;
use crate::configuration::{ApplicationKind, Decode, Format, Location, LocationInfo};
use log::{debug, warn};
use protobuf::{well_known_types, Message};
use proxy_wasm::traits::Context;
use thiserror::Error;
use threescalers::{
    api_call::{ApiCall, Kind},
    application::Application,
    credentials::Credentials,
    extensions,
    http::Request,
    service::Service,
    transaction::Transaction,
    usage::Usage,
};

#[derive(Debug, Error)]
enum MatchError {
    #[error("no known service matched")]
    NoServiceMatched,
    #[error("no credentials found in request")]
    CredentialsNotFound,
}

#[derive(Debug, Error)]
enum UnimplementedError {
    #[error("unimplemented credentials kind {0:#?}")]
    CredentialsKind(ApplicationKind),
}

fn parse_location<'a>(
    ctx: &'a HttpAuthThreescale,
    location_info: &LocationInfo,
    global_keys: &[&str],
    rh: &'a RequestHeaders,
    url: &'a url::Url,
) -> Option<(Value<'a>, Option<Format>)> {
    match location_info.location() {
        Location::QueryString { keys, decode } => {
            let mut keys = keys.iter().map(std::ops::Deref::deref).collect::<Vec<_>>();
            keys.extend(global_keys.iter());
            keys.iter().find_map(|&key| {
                url.query_pairs().find_map(|(k, v)| {
                    if key == k.as_ref() {
                        match Value::String(v).decode_multiple(decode.as_ref()) {
                            Ok(v) => Ok(v),
                            Err(e) => {
                                warn!("Error decoding query_string {:#?}", e);
                                Err(e)
                            }
                        }
                        .ok()
                        .map(|v| (v, Some(Format::String)))
                    } else {
                        None
                    }
                })
            })
        }
        Location::Header { keys, decode } => keys
            .iter()
            .find_map(|key| rh.get(key))
            .map(std::borrow::Cow::from)
            .map(|v| {
                match Value::String(v).decode_multiple(decode.as_ref()) {
                    Ok(v) => Ok(v),
                    Err(e) => {
                        warn!("Error decoding header {:#?}", e);
                        Err(e)
                    }
                }
                .ok()
                .map(|v| (v, Some(Format::String)))
            })
            .flatten(),
        Location::Property {
            path,
            format,
            lookup,
            decode,
        } => {
            let path = path.iter().map(|ps| ps.as_str()).collect::<Vec<_>>();
            let path_s = path.join("/");
            debug!("Looking up property path {}", path_s);
            if let Some(property) = ctx.get_property(path) {
                let b = property.as_slice();
                let ss = b.iter().map(|&b| b).collect::<Vec<_>>();

                match Value::Bytes(std::borrow::Cow::from(ss)).decode_multiple(decode.as_ref()) {
                    Ok(v) => Ok(v),
                    Err(e) => {
                        warn!("Error decoding property for {}", path_s);
                        Err(e)
                    }
                }
                .ok()
                .map(|v| (v, Some(*format)))
            } else {
                debug!("Property path not found {}", path_s);
                None
            }
        }
    }
}

pub(crate) fn authrep_request(
    ctx: &HttpAuthThreescale,
    rh: &RequestHeaders,
) -> Result<Request, anyhow::Error> {
    let (svc, kind, app_id, format, usages) = authrep(ctx, rh)?;
    build_call(svc, kind, app_id, format, usages)
}
pub(crate) fn authrep<'a>(
    ctx: &'a HttpAuthThreescale,
    //config: &Configuration,
    rh: &RequestHeaders,
) -> Result<
    (
        &'a crate::configuration::Service,
        ApplicationKind,
        String,
        Option<Format>,
        std::collections::HashMap<&'a str, i64>,
    ),
    anyhow::Error,
> {
    let config = ctx.configuration();
    let svclist = config.get_services()?;

    let metadata = rh.metadata();
    let method = metadata.method();
    let url = rh.url()?;
    let authority = url.authority();
    let path = url.path();

    let svc = svclist
        .iter()
        .find(|&svc| svc.match_authority(authority))
        .ok_or(MatchError::NoServiceMatched)?;

    let credentials = svc.credentials()?;

    let ((value, format), kind) = credentials
        .iter()
        .find_map(|param| {
            let kind = param.kind();
            let keys = param.keys();
            param
                .locations()
                .iter()
                .find_map(|location_info| -> Option<(Value, Option<Format>)> {
                    parse_location(
                        ctx,
                        location_info,
                        keys.iter()
                            .map(std::ops::Deref::deref)
                            .collect::<Vec<_>>()
                            .as_slice(),
                        rh,
                        &url,
                    )
                })
                .map(|value| (value, kind))
        })
        .ok_or(MatchError::CredentialsNotFound)?;

    debug!(
        "Found credentials, kind {:#?} format {:?} value {:#?}",
        kind, format, value
    );
    // XXX unwrap can panic here
    let value = value.to_string().unwrap();

    let mut usages = std::collections::HashMap::new();
    for rule in svc.mapping_rules() {
        debug!("matching rule {:#?}", rule);
        if method == rule.method().to_ascii_uppercase().as_str() && rule.match_pattern(path) {
            debug!("matched pattern in {}", path);
            for usage in rule.usages() {
                let value = usages.entry(usage.name()).or_insert(0);
                *value += usage.delta();
            }
        }
    }

    Ok((svc, kind, value, format, usages))
}

pub(crate) fn build_call(
    service: &crate::configuration::Service,
    kind: ApplicationKind,
    app_id: String,
    _format: Option<Format>,
    usages: std::collections::HashMap<&str, i64>,
) -> Result<Request, anyhow::Error> {
    let app = match kind {
        ApplicationKind::UserKey => Application::UserKey(app_id.into()),
        ApplicationKind::AppId | ApplicationKind::OIDC => Application::AppId(app_id.into(), None),
        k => anyhow::bail!(UnimplementedError::CredentialsKind(k)),
    };

    let usage = usages
        .into_iter()
        .map(|(k, v)| (k, format!("{}", v)))
        .collect::<Vec<_>>();
    let usage = Usage::new(usage.as_slice());
    let txn = Transaction::new(&app, None, Some(&usage), None);
    let txns = vec![txn];
    let extensions = extensions::List::new().no_body();

    let service = Service::new(
        service.id(),
        Credentials::ServiceToken(service.token().into()),
    );
    let mut apicall = ApiCall::builder(&service);
    // the builder here can only fail if we fail to set a kind
    let apicall = apicall
        .transactions(&txns)
        .extensions(&extensions)
        .kind(Kind::AuthRep)
        .build()?;

    Ok(Request::from(&apicall))
}
