use log::{error, info, warn};
use proxy_wasm::traits::*;
use proxy_wasm::types::*;

mod authrep;
mod configuration;
mod url;
use configuration::Configuration;

pub(crate) use crate::url::Url;

struct HttpAuthThreescale {
    context_id: u32,
    configuration: Configuration,
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
        let backend_url = backend.url();
        let backend_authority = match crate::url::authority(backend_url) {
            // authority is required when serializing, so this should not happen
            None => {
                error!("no authority found in configured 3scale backend - ensure it is a well formed URL including the scheme");
                return FilterHeadersStatus::Continue;
            }
            Some(authority) => authority,
        };

        let request = match authrep::authrep_request(self) {
            Err(e) => {
                error!("error computing authrep request {:?}", e);
                self.send_http_response(403, vec![], Some(b"Access forbidden.\n"));
                return FilterHeadersStatus::StopIteration;
            }
            Ok(request) => request,
        };

        // uri will actually just get the whole path + parameters
        let (mut uri, body) = request.uri_and_body();

        let backend_path = backend_url.path();
        // add in any path included in the backend URL
        // Note: this currently does not take into account any query parameters, but I've yet to see someone using those
        //       for proper support we'd likely want to have the client crate handle those
        if backend_path.len() > 1 {
            uri.to_mut().insert_str(0, backend_path);
        }

        let metadata = vec![
            (":method", request.method.as_str()),
            (":path", uri.as_ref()),
            (":authority", backend_authority.as_str()),
        ];
        let headers = request
            .headers
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .chain(metadata.into_iter())
            .collect::<Vec<_>>();

        let call_token = self
            .dispatch_http_call(
                backend.cluster_name(),
                headers,
                body.map(str::as_bytes),
                vec![],
                backend.timeout(),
            )
            .unwrap();

        info!("on_http_request_headers: call token is {}", call_token);

        FilterHeadersStatus::StopIteration
    }

    fn on_http_response_headers(&mut self, _: usize) -> FilterHeadersStatus {
        self.set_http_response_header("Powered-By", Some("3scale"));
        FilterHeadersStatus::Continue
    }
}

impl Context for HttpAuthThreescale {
    fn on_http_call_response(&mut self, call_token: u32, _: usize, _: usize, _: usize) {
        info!("on_http_call_response: call_token is {}", call_token);
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

        let conf = proxy_wasm::hostcalls::get_buffer(
            BufferType::PluginConfiguration,
            0,
            plugin_configuration_size,
        );

        if let Err(e) = conf {
            error!(
                "on_configure: error retrieving plugin configuration: {:#?}",
                e
            );
            return false;
        }

        let conf = conf.unwrap();
        if conf.is_none() {
            warn!("on_configure: empty plugin configuration");
            return true;
        }

        let conf = conf.unwrap();
        info!(
            "on_configure: raw config is {}",
            String::from_utf8_lossy(conf.as_slice())
        );

        let conf = Configuration::try_from(conf.as_slice());
        if let Err(e) = conf {
            error!("on_configure: error parsing plugin configuration {}", e);
            return false;
        }

        self.configuration = conf.unwrap().into();
        info!(
            "on_configure: plugin configuration {:#?}",
            self.configuration
        );

        true
    }

    fn on_create_child_context(&mut self, context_id: u32) -> Option<ChildContext> {
        info!("creating new context {}", context_id);
        let ctx = HttpAuthThreescale {
            context_id,
            configuration: self.configuration.as_ref().unwrap().clone(),
        };

        Some(ChildContext::HttpContext(Box::new(ctx)))
    }
}

#[no_mangle]
pub fn _start() {
    proxy_wasm::set_log_level(LogLevel::Trace);
    proxy_wasm::set_root_context(|_| -> Box<dyn RootContext> {
        Box::new(RootAuthThreescale::new())
    });
}
