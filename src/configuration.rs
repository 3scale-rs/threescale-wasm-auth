#![allow(dead_code)]

use crate::upstream::Upstream;
use core::convert::TryFrom;
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
    upstream: Upstream,
    token: String,
}

impl System {
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn upstream(&self) -> &Upstream {
        &self.upstream
    }

    pub fn token(&self) -> &str {
        self.token.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Backend {
    name: Option<String>,
    upstream: Upstream,
    extensions: Option<Vec<String>>,
}

impl Backend {
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn upstream(&self) -> &Upstream {
        &self.upstream
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
    #[serde(rename = "jwt_claims")]
    JWTClaims,
    Any,
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ApplicationKind {
    UserKey,
    AppId,
    AppKey,
    #[serde(rename = "oidc")]
    OIDC,
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

#[cfg(test)]
mod test {
    use super::*;

    mod fixtures {
        pub const CONFIG: &str = r#"{
            "system": {
              "upstream": {
                "name": "outbound|443||multitenant.3scale.net",
                "url": "https://istiodevel-admin.3scale.net",
                "timeout": 5000
              },
              "token": "invalid-token"
            },
            "backend": {
              "upstream": {
                "name": "outbound|443||su1.3scale.net",
                "url": "https://su1.3scale.net",
                "timeout": 5000
              }
            },
            "services": [
              {
                "id": "2555417834780",
                "token": "invalid-token",
                "authorities": [
                  "web",
                  "web.app",
                  "0.0.0.0:8080"
                ],
                "credentials": [
                  {
                    "kind": "user_key",
                    "key": "x-api-key",
                    "locations": [
                      "header",
                      "query_string"
                    ]
                  },
                  {
                    "kind": "oidc",
                    "key": "azp",
                    "locations": [
                      "jwt_claims"
                    ]
                  }
                ],
                "mapping_rules": [
                  {
                    "method": "get",
                    "pattern": "/",
                    "usages": [
                      {
                        "name": "hits",
                        "delta": 1
                      }
                    ]
                  },
                  {
                    "method": "get",
                    "pattern": "/productpage",
                    "usages": [
                      {
                        "name": "ticks",
                        "delta": 1
                      }
                    ]
                  }
                ]
              }
            ]
        }"#;
    }

    fn parse_config(input: &str) -> Configuration {
        let parsed = serde_json::from_str::<'_, Configuration>(input);
        match parsed {
            Err(ref e) => eprintln!("Error: {:#?}", e),
            _ => (),
        }
        assert!(parsed.is_ok());
        let parsed = parsed.unwrap();
        eprintln!("PARSED:\n{:#?}", parsed);
        parsed
    }

    #[test]
    fn it_parses_a_configuration_string() {
        parse_config(fixtures::CONFIG);
    }
}
