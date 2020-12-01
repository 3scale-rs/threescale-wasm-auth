use log::{error, info, warn};
use proxy_wasm::traits::*;
use proxy_wasm::types::*;

mod authrep;
mod configuration;
mod upstream;
mod url;

use configuration::Configuration;

struct HttpAuthThreescale {
    context_id: u32,
    configuration: Configuration,
}

impl HttpContext for HttpAuthThreescale {
    fn on_http_request_headers(&mut self, _: usize) -> FilterHeadersStatus {
        warn!("on_http_request_headers: context_id {}", self.context_id);
        warn!("=== BEGIN HEADERS ===");
        for (k, v) in self.get_http_request_headers().into_iter() {
            warn!("{}: {}", k, v);
        }
        warn!("===  END HEADERS  ===");

        let backend = match self.configuration.get_backend() {
            Err(e) => {
                error!("error obtaining configuration for 3scale backend: {:?}", e);
                return FilterHeadersStatus::Continue;
            }
            Ok(backend) => backend,
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
                // XXX NB this causes the client request to HANG and never get responded!
                return FilterHeadersStatus::StopIteration;
            }
        };

        warn!("on_http_request_headers: call token is {}", call_token);

        self.add_http_request_header("x-3scale-status", "pre-authorized");

        FilterHeadersStatus::StopIteration
    }

    fn on_http_response_headers(&mut self, _: usize) -> FilterHeadersStatus {
        self.set_http_response_header("Powered-By", Some("3scale"));
        FilterHeadersStatus::Continue
    }
}

impl Context for HttpAuthThreescale {
    fn on_http_call_response(&mut self, call_token: u32, _: usize, body_size: usize, _: usize) {
        warn!("on_http_call_response: call_token is {}", call_token);
        let headers = self.get_http_call_response_headers();
        warn!("on_http_call_response: headers {:?}", headers);
        let body = self.get_http_call_response_body(0, body_size);
        if let Some(body_b) = body {
            warn!("on_http_call_response: body {:?}", String::from_utf8_lossy(body_b.as_slice()));
        } else {
            warn!("on_http_call_response: body None");
        }

        let authorized = headers
            .into_iter()
            .find(|(key, _)| key.as_str() == ":status")
            .map(|(_, value)| value.as_str() == "200")
            .unwrap_or(false);

        self.add_http_request_header("x-3scale-auth", "granted");
        if authorized {
            warn!("on_http_call_response: authorized call_token {}", call_token);
            self.resume_http_request();
        } else {
            warn!("on_http_call_response: forbidden call_token {}", call_token);
            self.send_http_response(403, vec![], Some(b"Access forbidden.\n"));
        }
    }
}

pub fn proxy_log(msg: &str) {
    let _ = proxy_wasm::hostcalls::log(LogLevel::Critical, msg);
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
        proxy_log("starting 3scale Auth WASM extension");
        error!("on_vm_start: testing error log");
        warn!("on_vm_start: testing warn log");
        info!("on_vm_start: testing info log");
        warn!(
            "on_vm_start: vm_configuration_size is {}",
            vm_configuration_size
        );
        proxy_log(
            format!("on_vm_start: vm_configuration_size is {}",
            vm_configuration_size).as_str()
        );
        // THIS BREAKS ON SERVICE MESH 2.0 (Istio 1.6) -- likely needs get_vm_configuration
        //let vm_config = proxy_wasm::hostcalls::get_buffer(
        //    BufferType::VmConfiguration,
        //    0,
        //    vm_configuration_size,
        //);

        //if let Err(e) = vm_config {
        //    error!("on_vm_start: error retrieving VM configuration: {:#?}", e);
        //    return false;
        //}

        //self.vm_configuration = vm_config.unwrap();

        //if let Some(conf) = self.vm_configuration.as_ref() {
        //    info!(
        //        "on_vm_start: VM configuration is {}",
        //        core::str::from_utf8(conf).unwrap()
        //    );
        //    true
        //} else {
        //    warn!("on_vm_start: empty VM config");
        //    false
        //}
        true
    }

    fn on_configure(&mut self, plugin_configuration_size: usize) -> bool {
        use core::convert::TryFrom;

        warn!(
            "on_configure: plugin_configuration_size is {}",
            plugin_configuration_size
        );

        // DOES NOT WORK ON SM 2.0 / Istio 1.6
        //let conf = proxy_wasm::hostcalls::get_buffer(
        //    BufferType::PluginConfiguration,
        //    0,
        //    plugin_configuration_size,
        //);
        let conf = proxy_wasm::hostcalls::get_configuration();

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
        warn!(
            "on_configure: raw config is {}",
            String::from_utf8_lossy(conf.as_slice())
        );

        let conf = Configuration::try_from(conf.as_slice());
        if let Err(e) = conf {
            error!("on_configure: error parsing plugin configuration {}", e);
            return false;
        }

        self.configuration = conf.unwrap().into();
        warn!(
            "on_configure: plugin configuration {:#?}",
            self.configuration
        );

        true
    }

    fn on_create_child_context(&mut self, context_id: u32) -> Option<ChildContext> {
        warn!("creating new context {}", context_id);
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
