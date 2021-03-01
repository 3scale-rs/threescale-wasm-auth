mod authrep;
mod decode;
mod request_headers;

use log::{debug, error, info, warn};
use proxy_wasm::traits::*;
use proxy_wasm::types::*;

use crate::configuration::Configuration;

pub(crate) struct HttpAuthThreescale {
    context_id: u32,
    configuration: Configuration,
}

impl HttpAuthThreescale {
    pub fn configuration(&self) -> &Configuration {
        &self.configuration
    }
}

impl HttpContext for HttpAuthThreescale {
    fn on_http_request_headers(&mut self, _: usize) -> FilterHeadersStatus {
        info!("on_http_request_headers: context_id {}", self.context_id);
        let backend = match self.configuration.get_backend() {
            Err(e) => {
                error!("error obtaining configuration for 3scale backend: {:?}", e);
                return FilterHeadersStatus::Continue;
            }
            Ok(backend) => backend,
        };

        let rh = request_headers::RequestHeaders::new(self);
        let request = match authrep::authrep_request(self, &rh) {
            Err(e) => {
                error!("error computing authrep request {:?}", e);
                self.send_http_response(403, vec![], Some(b"Access forbidden.\n"));
                info!("threescale_wasm_auth: 403 sent");
                return FilterHeadersStatus::StopIteration;
            }
            Ok(request) => request,
        };

        // uri will actually just get the whole path + parameters
        let (uri, body) = request.uri_and_body();

        let headers = request
            .headers
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect::<Vec<_>>();

        let upstream = backend.upstream();
        let call_token = match upstream.call(
            self,
            uri.as_ref(),
            request.method.as_str(),
            headers,
            body.map(str::as_bytes),
            None,
            None,
        ) {
            Ok(call_token) => call_token,
            Err(e) => {
                error!("on_http_request_headers: could not dispatch HTTP call to {}: did you create the cluster to do so? - {:#?}", upstream.name(), e);
                return FilterHeadersStatus::StopIteration;
            }
        };

        info!(
            "threescale_wasm_auth: on_http_request_headers: call token is {}",
            call_token
        );

        FilterHeadersStatus::StopIteration
    }

    fn on_http_response_headers(&mut self, _: usize) -> FilterHeadersStatus {
        self.set_http_response_header("Powered-By", Some("3scale"));
        FilterHeadersStatus::Continue
    }
}

impl Context for HttpAuthThreescale {
    fn on_http_call_response(&mut self, call_token: u32, _: usize, _: usize, _: usize) {
        info!(
            "threescale_wasm_auth: on_http_call_response: call_token is {}",
            call_token
        );
        let authorized = self
            .get_http_call_response_headers()
            .into_iter()
            .find(|(key, _)| key.as_str() == ":status")
            .map(|(_, value)| value.as_str() == "200")
            .unwrap_or(false);

        if authorized {
            info!("on_http_call_response: authorized {}", call_token);
            self.resume_http_request();
        } else {
            info!("on_http_call_response: forbidden {}", call_token);
            self.send_http_response(403, vec![], Some(b"Access forbidden.\n"));
            info!("threescale_wasm_auth: 403 sent");
        }
    }
}

struct RootAuthThreescale {
    vm_configuration: Option<Vec<u8>>,
    configuration: Option<Configuration>,
}

impl RootAuthThreescale {
    pub fn new() -> Self {
        Self {
            vm_configuration: None,
            configuration: None,
        }
    }
}

impl Context for RootAuthThreescale {}

impl RootContext for RootAuthThreescale {
    fn on_vm_start(&mut self, vm_configuration_size: usize) -> bool {
        info!(
            "on_vm_start: vm_configuration_size is {}",
            vm_configuration_size
        );
        let vm_config = proxy_wasm::hostcalls::get_buffer(
            BufferType::VmConfiguration,
            0,
            vm_configuration_size,
        );

        if let Err(e) = vm_config {
            error!("on_vm_start: error retrieving VM configuration: {:#?}", e);
            return false;
        }

        self.vm_configuration = vm_config.unwrap();

        if let Some(conf) = self.vm_configuration.as_ref() {
            info!(
                "on_vm_start: VM configuration is {}",
                core::str::from_utf8(conf).unwrap()
            );
            true
        } else {
            warn!("on_vm_start: empty VM config");
            false
        }
    }

    fn on_configure(&mut self, plugin_configuration_size: usize) -> bool {
        use core::convert::TryFrom;

        info!(
            "on_configure: plugin_configuration_size is {}",
            plugin_configuration_size
        );

        let conf = match proxy_wasm::hostcalls::get_buffer(
            BufferType::PluginConfiguration,
            0,
            plugin_configuration_size,
        ) {
            Ok(Some(conf)) => conf,
            Ok(None) => {
                warn!("empty module configuration - module has no effect");
                return true;
            }
            Err(e) => {
                error!("error retrieving module configuration: {:#?}", e);
                return false;
            }
        };

        debug!("loaded raw config");

        let conf = match Configuration::try_from(conf.as_slice()) {
            Ok(conf) => conf,
            Err(e) => {
                let conf_str = String::from_utf8_lossy(conf.as_slice());
                for line in crate::util::serde_json_error_lines(&e, conf_str.as_ref()) {
                    error!("{}", line);
                }
                return false;
            }
        };

        self.configuration = conf.into();
        info!(
            "on_configure: plugin configuration {:#?}",
            self.configuration
        );

        true
    }

    fn on_create_child_context(&mut self, context_id: u32) -> Option<ChildContext> {
        info!("threewscale_wasm_auth: creating new context {}", context_id);
        let ctx = HttpAuthThreescale {
            context_id,
            configuration: self.configuration.as_ref().unwrap().clone(),
        };

        Some(ChildContext::HttpContext(Box::new(ctx)))
    }
}

#[cfg_attr(
    all(
        target_arch = "wasm32",
        target_vendor = "unknown",
        target_os = "unknown"
    ),
    export_name = "_start"
)]
#[cfg_attr(
    not(all(
        target_arch = "wasm32",
        target_vendor = "unknown",
        target_os = "unknown"
    )),
    allow(dead_code)
)]
// This is a C interface, so make it explicit in the fn signature (and avoid mangling)
extern "C" fn start() {
    proxy_wasm::set_log_level(LogLevel::Trace);
    proxy_wasm::set_root_context(|_| -> Box<dyn RootContext> {
        Box::new(RootAuthThreescale::new())
    });
}
