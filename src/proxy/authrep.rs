use std::vec;

use super::request_headers::RequestHeaders;
use super::HttpAuthThreescale;
use crate::configuration::{ApplicationKind, Location};
use log::debug;
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

pub(crate) fn authrep_request(
    ctx: &HttpAuthThreescale,
    //config: &Configuration,
    rh: &RequestHeaders,
) -> Result<Request, anyhow::Error> {
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

    let (value, kind) =
        credentials
            .iter()
            .find_map(|param| {
                let kind = param.kind();
                let keys = param.keys();
                param
                    .locations()
                    .iter()
                    .find_map(|&location| match location {
                        Location::QueryString => keys.iter().find_map(|key| {
                            url.query_pairs().find_map(|(k, v)| {
                                if key == k.as_ref() {
                                    Some(v)
                                } else {
                                    None
                                }
                            })
                        }),
                        Location::Header => keys
                            .iter()
                            .find_map(|key| rh.get(key))
                            .map(std::borrow::Cow::from),
                        Location::JWTClaims => {
                            // parse an explicit metadata path to look for the claims
                            let path = param
                                .metadata()
                                .and_then(|metadata| {
                                    metadata.get("path").and_then(|path| match path.as_str() {
                                        Some(s) => Some(s.split('/').collect::<Vec<&str>>()),
                                        None => path
                                            .as_array()?
                                            .iter()
                                            .map(serde_json::Value::as_str)
                                            .collect::<Option<_>>(),
                                    })
                                })
                                .unwrap_or_else(|| {
                                    vec![
                                        "metadata",
                                        //"metadata_context",
                                        "filter_metadata",
                                        "envoy.filters.http.jwt_authn",
                                        "verified_jwt",
                                    ]
                                });
                            debug!("JWT path is {:?}", path);
                            keys.iter().find_map(|key| {
                                // unfortunately the proxy-wasm API requires us to keep cloning the base path.
                                let mut property_path = path.clone();
                                //property_path.push(key.as_str());
                                //let value = ctx.get_property(property_path).and_then(|v| {
                                let value_res = proxy_wasm::hostcalls::get_property(property_path).unwrap();
                                let value = value_res.as_ref().and_then(|v| {
                                    Some(String::from_utf8_lossy(v.as_slice()))
                                });
                                debug!("Checking lossy JWT Claim {:#?} => {:#?}", key, value);
                                debug!("value_res: {:#?}", value_res);
                                let value = value_res.and_then(|v| {
                                    String::from_utf8(v).map(std::borrow::Cow::from).ok()
                                });
                                debug!("Checking JWT Claim {:#?} => {:#?}", key, value);
                                value
                            })
                        }
                    })
                    .map(|value| (value, kind))
            })
            .ok_or(MatchError::CredentialsNotFound)?;

    debug!("Found credentials, kind {:#?} value {:#?}", kind, value);
    let app = match kind {
        ApplicationKind::UserKey => Application::UserKey(value.to_string().into()),
        ApplicationKind::AppId | ApplicationKind::OIDC => {
            Application::AppId(value.to_string().into(), None)
        }
        // TODO implement handling of additional kinds
        k => anyhow::bail!(UnimplementedError::CredentialsKind(k)),
    };

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

    let usage = usages
        .into_iter()
        .map(|(k, v)| (k, format!("{}", v)))
        .collect::<Vec<_>>();
    let usage = Usage::new(usage.as_slice());
    let txn = Transaction::new(&app, None, Some(&usage), None);
    let txns = vec![txn];
    let extensions = extensions::List::new().no_body();

    let service = Service::new(svc.id(), Credentials::ServiceToken(svc.token().into()));
    let mut apicall = ApiCall::builder(&service);
    // the builder here can only fail if we fail to set a kind
    let apicall = apicall
        .transactions(&txns)
        .extensions(&extensions)
        .kind(Kind::AuthRep)
        .build()
        .unwrap();

    Ok(Request::from(&apicall))
}
