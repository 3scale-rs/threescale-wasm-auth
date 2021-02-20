use super::request_headers::RequestHeaders;
use super::HttpAuthThreescale;
use crate::configuration::{ApplicationKind, Location};
use log::debug;
use proxy_wasm::traits::HttpContext;
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
    rh: &RequestHeaders,
) -> Result<Request, anyhow::Error> {
    let svclist = ctx.configuration().get_services()?;

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

    let (value, kind) = credentials
        .iter()
        .find_map(|param| {
            let key = param.key();
            let kind = param.kind();
            param
                .locations()
                .iter()
                .find_map(|&location| match location {
                    // TODO add more location impls
                    Location::Header => ctx.get_http_request_header(key),
                    _ => None,
                })
                .map(|value| (value, kind))
        })
        .ok_or(MatchError::CredentialsNotFound)?;

    let app = match kind {
        ApplicationKind::UserKey => Application::UserKey(value.into()),
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
