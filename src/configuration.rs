// XXX TODO avoid warnings for now on unused fns, since this is in progress
#![allow(dead_code)]

use core::convert::TryFrom;
use core::time::Duration;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum MissingError {
    #[error("no backend configured")]
    Backend,
    #[error("no services configured")]
    Services,
    #[error("no credentials defined for service `{0}`")]
    Credentials(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct System {
    name: Option<String>,
    cluster_name: String,
    #[serde(deserialize_with = "crate::url::deserialize")]
    url: crate::Url,
    timeout: f64,
    token: String,
}

impl System {
    pub fn cluster_name(&self) -> &str {
        self.cluster_name.as_str()
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn url(&self) -> &crate::Url {
        &self.url
    }

    pub fn timeout(&self) -> Duration {
        Duration::from_secs_f64(self.timeout)
    }

    pub fn token(&self) -> &str {
        self.token.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Backend {
    cluster_name: String,
    #[serde(deserialize_with = "crate::url::deserialize")]
    url: crate::Url,
    timeout: f64,
    extensions: Option<Vec<String>>,
}

impl Backend {
    pub fn cluster_name(&self) -> &str {
        self.cluster_name.as_str()
    }
    pub fn url(&self) -> &crate::Url {
        &self.url
    }

    pub fn timeout(&self) -> Duration {
        Duration::from_secs_f64(self.timeout)
    }

    pub fn extensions(&self) -> Option<&Vec<String>> {
        self.extensions.as_ref()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Location {
    Header,
    QueryString,
    Body,
    Trailer,
    Any,
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ApplicationKind {
    UserKey,
    AppId,
    AppKey,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Parameter<K> {
    locations: Vec<Location>,
    kind: ApplicationKind,
    key: K,
}

impl<K> Parameter<K> {
    pub fn locations(&self) -> &Vec<Location> {
        self.locations.as_ref()
    }

    pub fn kind(&self) -> ApplicationKind {
        self.kind
    }

    pub fn key(&self) -> &K {
        &self.key
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Service {
    id: String,
    token: String,
    authorities: Vec<String>,
    credentials: Vec<Parameter<String>>,
    mapping_rules: Vec<MappingRule>,
}

impl Service {
    pub fn id(&self) -> &str {
        self.id.as_str()
    }

    pub fn token(&self) -> &str {
        self.token.as_str()
    }

    pub fn authorities(&self) -> &Vec<String> {
        self.authorities.as_ref()
    }

    pub fn credentials(&self) -> Result<&Vec<Parameter<String>>, MissingError> {
        if self.credentials.is_empty() {
            Err(MissingError::Credentials(self.id.to_owned()))
        } else {
            Ok(self.credentials.as_ref())
        }
    }

    pub fn mapping_rules(&self) -> &Vec<MappingRule> {
        self.mapping_rules.as_ref()
    }

    pub fn match_authority(&self, authority: &str) -> bool {
        self.authorities.iter().any(|auth| auth == authority)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct MappingRule {
    method: String,
    pattern: String,
    usages: Vec<Usage>,
}

impl MappingRule {
    pub fn method(&self) -> &str {
        self.method.as_str()
    }

    pub fn pattern(&self) -> &str {
        self.pattern.as_str()
    }

    pub fn usages(&self) -> &Vec<Usage> {
        self.usages.as_ref()
    }

    pub fn match_pattern(&self, pattern: &str) -> bool {
        pattern.starts_with(&self.pattern)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Usage {
    name: String,
    delta: i64,
}

impl Usage {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn delta(&self) -> i64 {
        self.delta
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "3scale")]
pub(crate) struct Configuration {
    system: System,
    backend: Option<Backend>,
    services: Option<Vec<Service>>,
}

impl TryFrom<&[u8]> for Configuration {
    type Error = anyhow::Error;

    fn try_from(buf: &[u8]) -> Result<Self, Self::Error> {
        Ok(serde_json::from_slice(buf)?)
    }
}

impl Configuration {
    pub fn system(&self) -> &System {
        &self.system
    }

    pub fn backend(&self) -> Option<&Backend> {
        self.backend.as_ref()
    }

    pub fn services(&self) -> Option<&Vec<Service>> {
        self.services.as_ref()
    }

    pub fn get_backend(&self) -> Result<&Backend, MissingError> {
        self.backend().ok_or(MissingError::Backend)
    }

    pub fn get_services(&self) -> Result<&Vec<Service>, MissingError> {
        self.services().ok_or(MissingError::Services)
    }
}
