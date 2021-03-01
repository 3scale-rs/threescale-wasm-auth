#![allow(dead_code)]

use crate::upstream::Upstream;
use core::convert::TryFrom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

mod location;
pub(crate) use location::*;

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
pub(crate) enum ApplicationKind {
    UserKey,
    AppId,
    AppKey,
    #[serde(rename = "oidc")]
    OIDC,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Parameter<K> {
    locations: Vec<LocationInfo>,
    kind: ApplicationKind,
    keys: Vec<K>,
    #[serde(flatten)]
    other: HashMap<String, serde_json::Value>,
}

impl<K> Parameter<K> {
    pub fn locations(&self) -> &Vec<LocationInfo> {
        self.locations.as_ref()
    }

    pub fn kind(&self) -> ApplicationKind {
        self.kind
    }

    pub fn keys(&self) -> &Vec<K> {
        self.keys.as_ref()
    }

    pub fn other(&self) -> &HashMap<String, serde_json::Value> {
        &self.other
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
    type Error = serde_json::Error;

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct JWT {
    exp: u64,
    iat: u64,
    auth_time: u64,
    jti: String,
    iss: String,
    aud: String,
    sub: String,
    typ: String,
    azp: String,
    session_state: String,
    at_hash: String,
    acr: String,
    email_verified: bool,
    preferred_username: String,
}

#[cfg(test)]
mod test {
    use protobuf::Message;

    use super::*;

    mod fixtures {
        pub const JWT: &str = "eyJhbGciOiJSUzI1NiIsInR5cCIgOiAiSldUIiwia2lkIiA6ICJFVGoxbzI1SlcySS1BVnM1aThHeTMxeEtFUzYzd3lKM2NnUXBmUFBoOHh3In0.eyJleHAiOjE2MTQ2Mzk2NDQsImlhdCI6MTYxNDYzOTU4NCwiYXV0aF90aW1lIjoxNjE0NjM5NTgzLCJqdGkiOiJlOWUwZjMwYy03NTRhLTQ3MmMtOTQ5Ny1iN2Y2ZGJkMzZlNGUiLCJpc3MiOiJodHRwczovL2tleWNsb2FrOjg0NDMvYXV0aC9yZWFsbXMvbWFzdGVyIiwiYXVkIjoidGVzdCIsInN1YiI6ImVmZWMzMjIwLWM3OGMtNGNmNy04ZGZiLWU5ODkyZjBlYzIzZSIsInR5cCI6IklEIiwiYXpwIjoidGVzdCIsInNlc3Npb25fc3RhdGUiOiIzMzQ3NDE1Mi1mODgyLTRjOTYtOTc2YS1kMjI0M2I3ZGZjMzEiLCJhdF9oYXNoIjoiaGlCdG1ZSkpGWmZkWXlTZEJRdkdxQSIsImFjciI6IjEiLCJlbWFpbF92ZXJpZmllZCI6ZmFsc2UsInByZWZlcnJlZF91c2VybmFtZSI6ImFkbWluIn0.dLOE-kXjevLfrF5c5jMx2eK_2gmSlcPHT7XFrZSHIqE7DqCjs1JBUkNwh7428WIXdtMR65YX27naXS598cLcYjgEh8kZ_944KoUkDTUVHEsIhr4426w1DOILxedTaJrneCgjTgqwvevQX929Mq03XEmgqG2YTK_gT_LDDcT1fEGcMlrPEFF1920XBHbSQDHeyxM50OYKitt3w8UO0zGN_AfEs-JTwvFUVPKIZgoSVhEkjDowuW7qexeeScp2_tbA6lmqU6GivL9J9lOJr1JeBmIJmcUWoJ4bCH3Tt9o7wPfJDD7ps5OQjEN_6tswfTsKnvzRVWwdvU09DZx5oTjg9A";
        pub const JWT_PAYLOAD: &str = "eyJleHAiOjE2MTQ2Mzk2NDQsImlhdCI6MTYxNDYzOTU4NCwiYXV0aF90aW1lIjoxNjE0NjM5NTgzLCJqdGkiOiJlOWUwZjMwYy03NTRhLTQ3MmMtOTQ5Ny1iN2Y2ZGJkMzZlNGUiLCJpc3MiOiJodHRwczovL2tleWNsb2FrOjg0NDMvYXV0aC9yZWFsbXMvbWFzdGVyIiwiYXVkIjoidGVzdCIsInN1YiI6ImVmZWMzMjIwLWM3OGMtNGNmNy04ZGZiLWU5ODkyZjBlYzIzZSIsInR5cCI6IklEIiwiYXpwIjoidGVzdCIsInNlc3Npb25fc3RhdGUiOiIzMzQ3NDE1Mi1mODgyLTRjOTYtOTc2YS1kMjI0M2I3ZGZjMzEiLCJhdF9oYXNoIjoiaGlCdG1ZSkpGWmZkWXlTZEJRdkdxQSIsImFjciI6IjEiLCJlbWFpbF92ZXJpZmllZCI6ZmFsc2UsInByZWZlcnJlZF91c2VybmFtZSI6ImFkbWluIn0";
        pub const JWT_JSON: &str = r#"{
            "exp": 1614639644,
            "iat": 1614639584,
            "auth_time": 1614639583,
            "jti": "e9e0f30c-754a-472c-9497-b7f6dbd36e4e",
            "iss": "https://keycloak:8443/auth/realms/master",
            "aud": "test",
            "sub": "efec3220-c78c-4cf7-8dfb-e9892f0ec23e",
            "typ": "ID",
            "azp": "test",
            "session_state": "33474152-f882-4c96-976a-d2243b7dfc31",
            "at_hash": "hiBtmYJJFZfdYySdBQvGqA",
            "acr": "1",
            "email_verified": false,
            "preferred_username": "admin"
        }"#;
        pub const PROTOBUF_BIN: &[u8] = &[
            0x61, 0x74, 0x5f, 0x68, 0x61, 0x73, 0x68, 0x0, 0x68, 0x69, 0x42, 0x74, 0x6d, 0x59,
            0x4a, 0x4a, 0x46, 0x5a, 0x66, 0x64, 0x59, 0x79, 0x53, 0x64, 0x42, 0x51, 0x76, 0x47,
            0x71, 0x41, 0x0, 0x65, 0x78, 0x70, 0x0, 0x0, 0x0, 0x0, 0x87, 0x5c, 0xf, 0xd8, 0x41,
            0x0, 0x61, 0x75, 0x64, 0x0, 0x74, 0x65, 0x73, 0x74, 0x0, 0x73, 0x75, 0x62, 0x0, 0x65,
            0x66, 0x65, 0x63, 0x33, 0x32, 0x32, 0x30, 0x2d, 0x63, 0x37, 0x38, 0x63, 0x2d, 0x34,
            0x63, 0x66, 0x37, 0x2d, 0x38, 0x64, 0x66, 0x62, 0x2d, 0x65, 0x39, 0x38, 0x39, 0x32,
            0x66, 0x30, 0x65, 0x63, 0x32, 0x33, 0x65, 0x0, 0x6a, 0x74, 0x69, 0x0, 0x65, 0x39, 0x65,
            0x30, 0x66, 0x33, 0x30, 0x63, 0x2d, 0x37, 0x35, 0x34, 0x61, 0x2d, 0x34, 0x37, 0x32,
            0x63, 0x2d, 0x39, 0x34, 0x39, 0x37, 0x2d, 0x62, 0x37, 0x66, 0x36, 0x64, 0x62, 0x64,
            0x33, 0x36, 0x65, 0x34, 0x65, 0x0, 0x61, 0x75, 0x74, 0x68, 0x5f, 0x74, 0x69, 0x6d,
            0x65, 0x0, 0x0, 0x0, 0xc0, 0x77, 0x5c, 0xf, 0xd8, 0x41, 0x0, 0x65, 0x6d, 0x61, 0x69,
            0x6c, 0x5f, 0x76, 0x65, 0x72, 0x69, 0x66, 0x69, 0x65, 0x64, 0x0, 0x0, 0x0, 0x61, 0x7a,
            0x70, 0x0, 0x74, 0x65, 0x73, 0x74, 0x0, 0x73, 0x65, 0x73, 0x73, 0x69, 0x6f, 0x6e, 0x5f,
            0x73, 0x74, 0x61, 0x74, 0x65, 0x0, 0x33, 0x33, 0x34, 0x37, 0x34, 0x31, 0x35, 0x32,
            0x2d, 0x66, 0x38, 0x38, 0x32, 0x2d, 0x34, 0x63, 0x39, 0x36, 0x2d, 0x39, 0x37, 0x36,
            0x61, 0x2d, 0x64, 0x32, 0x32, 0x34, 0x33, 0x62, 0x37, 0x64, 0x66, 0x63, 0x33, 0x31,
            0x0, 0x69, 0x73, 0x73, 0x0, 0x68, 0x74, 0x74, 0x70, 0x73, 0x3a, 0x2f, 0x2f, 0x6b, 0x65,
            0x79, 0x63, 0x6c, 0x6f, 0x61, 0x6b, 0x3a, 0x38, 0x34, 0x34, 0x33, 0x2f, 0x61, 0x75,
            0x74, 0x68, 0x2f, 0x72, 0x65, 0x61, 0x6c, 0x6d, 0x73, 0x2f, 0x6d, 0x61, 0x73, 0x74,
            0x65, 0x72, 0x0, 0x69, 0x61, 0x74, 0x0, 0x0, 0x0, 0x0, 0x78, 0x5c, 0xf, 0xd8, 0x41,
            0x0, 0x61, 0x63, 0x72, 0x0, 0x31, 0x0, 0x70, 0x72, 0x65, 0x66, 0x65, 0x72, 0x72, 0x65,
            0x64, 0x5f, 0x75, 0x73, 0x65, 0x72, 0x6e, 0x61, 0x6d, 0x65, 0x0, 0x61, 0x64, 0x6d,
            0x69, 0x6e, 0x0, 0x74, 0x79, 0x70, 0x0, 0x49, 0x44, 0x0,
        ];
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
                    "keys": ["x-api-key"],
                    "locations": [
                      "header",
                      "query_string"
                    ]
                  },
                  {
                    "kind": "oidc",
                    "keys": ["aud", "azp"],
                    "locations": [
                        {
                            "location": { "property": ["one", "two"] }
                        }
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
            Err(ref e) => eprintln!("{}", crate::util::serde_json_error_to_string(e, input)),
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

    fn get_config() -> Configuration {
        Configuration {
            system: System {
                name: Some("system-name".into()),
                upstream: Upstream {
                    name: "outbound|443||multitenant.3scale.net".into(),
                    url: "https://istiodevel-admin.3scale.net".parse().unwrap(),
                    timeout: core::time::Duration::from_millis(5000),
                },
                token: "atoken".into(),
            },
            backend: Some(Backend {
                name: Some("backend-name".into()),
                upstream: Upstream {
                    name: "outbound|443||su1.3scale.net".into(),
                    url: "https://su1.3scale.net".parse().unwrap(),
                    timeout: core::time::Duration::from_millis(5000),
                },
                extensions: Some(vec!["no_body".to_string()]),
            }),
            services: Some(vec![Service {
                id: "2555417834780".into(),
                token: "service_token".into(),
                authorities: vec!["0.0.0.0:8080".into(), "0.0.0.0:8443".into()],
                credentials: vec![Parameter::<String> {
                    other: HashMap::new(),
                    kind: ApplicationKind::OIDC,
                    keys: vec!["azp".into(), "aud".into(), "x-jwt-payload".into()],
                    locations: vec![
                        LocationInfo {
                            location: Location::Header,
                            path: None,
                            value_dnf: ValueDnF {
                                decode: Some(vec![Decode::Base64Decode, Decode::JsonValue]),
                                format: Some(Format::Json),
                            },
                        },
                        LocationInfo {
                            location: Location::Property,
                            path: Some(vec![
                                "metadata".into(),
                                "filter_metadata".into(),
                                "envoy.filters.http.jwt_authn".into(),
                                "verified_jwt".into(),
                            ]),
                            value_dnf: ValueDnF {
                                decode: Some(vec![Decode::ProtobufValue]),
                                format: None,
                            },
                        },
                        LocationInfo {
                            location: Location::Property,
                            path: None,
                            value_dnf: ValueDnF {
                                decode: Some(vec![Decode::ProtobufValue]),
                                format: None,
                            },
                        },
                    ],
                }],
                mapping_rules: vec![MappingRule {
                    method: "GET".into(),
                    pattern: "/".into(),
                    usages: vec![Usage {
                        name: "Hits".into(),
                        delta: 1,
                    }],
                }],
            }]),
        }
    }

    #[test]
    fn print_config() {
        let config = get_config();
        let str = serde_json::to_string_pretty(&config);
        match &str {
            Err(e) => eprintln!("Failed to serialize configuration: {:#?}", e),
            Ok(s) => println!("{}", s),
        }
        assert!(str.is_ok());
    }

    #[test]
    fn jwt_parse() {
        let jwt = "eyJhbGciOiJSUzI1NiIsInR5cCIgOiAiSldUIiwia2lkIiA6ICJFVGoxbzI1SlcySS1BVnM1aThHeTMxeEtFUzYzd3lKM2NnUXBmUFBoOHh3In0.eyJleHAiOjE2MTQ2Mzk2NDQsImlhdCI6MTYxNDYzOTU4NCwiYXV0aF90aW1lIjoxNjE0NjM5NTgzLCJqdGkiOiJlOWUwZjMwYy03NTRhLTQ3MmMtOTQ5Ny1iN2Y2ZGJkMzZlNGUiLCJpc3MiOiJodHRwczovL2tleWNsb2FrOjg0NDMvYXV0aC9yZWFsbXMvbWFzdGVyIiwiYXVkIjoidGVzdCIsInN1YiI6ImVmZWMzMjIwLWM3OGMtNGNmNy04ZGZiLWU5ODkyZjBlYzIzZSIsInR5cCI6IklEIiwiYXpwIjoidGVzdCIsInNlc3Npb25fc3RhdGUiOiIzMzQ3NDE1Mi1mODgyLTRjOTYtOTc2YS1kMjI0M2I3ZGZjMzEiLCJhdF9oYXNoIjoiaGlCdG1ZSkpGWmZkWXlTZEJRdkdxQSIsImFjciI6IjEiLCJlbWFpbF92ZXJpZmllZCI6ZmFsc2UsInByZWZlcnJlZF91c2VybmFtZSI6ImFkbWluIn0.dLOE-kXjevLfrF5c5jMx2eK_2gmSlcPHT7XFrZSHIqE7DqCjs1JBUkNwh7428WIXdtMR65YX27naXS598cLcYjgEh8kZ_944KoUkDTUVHEsIhr4426w1DOILxedTaJrneCgjTgqwvevQX929Mq03XEmgqG2YTK_gT_LDDcT1fEGcMlrPEFF1920XBHbSQDHeyxM50OYKitt3w8UO0zGN_AfEs-JTwvFUVPKIZgoSVhEkjDowuW7qexeeScp2_tbA6lmqU6GivL9J9lOJr1JeBmIJmcUWoJ4bCH3Tt9o7wPfJDD7ps5OQjEN_6tswfTsKnvzRVWwdvU09DZx5oTjg9A";
        let jwt_parts = jwt.split('.').collect::<Vec<_>>();
        assert_eq!(jwt_parts.len(), 3);
        let jwt_first = base64::decode_config(jwt_parts[0], base64::URL_SAFE);
        assert!(jwt_first.is_ok());
        let jwt_first = jwt_first.unwrap();
        // generate message with something like prost::json::StringToMessage(&jwt_first)
        let jwt_first_s = unsafe { String::from_utf8_unchecked(jwt_first) };
        let jwt_first_pb = protobuf::json::parse_from_str::<protobuf::well_known_types::Struct>(
            jwt_first_s.as_str(),
        );
        assert!(jwt_first_pb.is_ok());
        let jwt_first_pb = jwt_first_pb.unwrap();
        let jwt_first_fields = &jwt_first_pb.fields;
        let alg = jwt_first_fields.get("alg");
        assert!(alg.is_some());
        let alg = alg.unwrap();
        match &alg.kind {
            Some(protobuf::well_known_types::value::Kind::string_value(s)) => {
                eprintln!("matching the value of alg is {}", s)
            }
            Some(v) => {
                eprintln!("value which should have been string is not! it is {:#?}", v);
            }
            None => (),
        }
        assert!(alg.has_string_value());
        let alg_s = alg.get_string_value();
        eprintln!("alg is {}", alg_s);
        // kid if present must be a string
        match jwt_first_fields.get("kid") {
            Some(kid) => {
                assert!(kid.has_string_value());
                eprintln!("kid is {}", kid.get_string_value());
            }
            None => eprintln!("kid not found"),
        }
        // typ should be JWT
        match jwt_first_fields.get("typ") {
            Some(typ) => {
                assert!(typ.has_string_value());
                eprintln!("typ is {}", typ.get_string_value());
            }
            None => eprintln!("typ not found"),
        }
        let jwt_payload = base64::decode_config(jwt_parts[1], base64::URL_SAFE);
        assert!(jwt_payload.is_ok());
        let jwt_payload = jwt_payload.unwrap();
        let jwt_payload_s = unsafe { String::from_utf8_unchecked(jwt_payload) };
        let jwt_payload_pb = protobuf::json::parse_from_str::<protobuf::well_known_types::Struct>(
            jwt_payload_s.as_str(),
        );
        assert!(jwt_payload_pb.is_ok());
        let jwt_payload_pb = jwt_payload_pb.unwrap();
        let bytes_out = jwt_payload_pb.write_to_bytes();
        assert!(
            bytes_out.is_ok(),
            "cannot create bytes vector out of payload pb"
        );
        let bytes_out = bytes_out.unwrap();
        let hex = bytes_out
            .iter()
            .map(|c| format!("{:#02x}", *c))
            .collect::<Vec<_>>()
            .join(", ");
        eprintln!("Payload PB bytes (len {}): [{}]", bytes_out.len(), hex);
        let hex = fixtures::PROTOBUF_BIN
            .iter()
            .map(|c| format!("{:#02x}", *c))
            .collect::<Vec<_>>()
            .join(", ");
        eprintln!(
            "Original Payload PB bytes (len {}): [{}]",
            fixtures::PROTOBUF_BIN.len(),
            hex
        );
        let jwt_payload_fields = &jwt_payload_pb.fields;
        ["iss", "sub", "jti"]
            .iter()
            .for_each(|&f| match jwt_payload_fields.get(f) {
                Some(v) => {
                    assert!(v.has_string_value());
                    eprintln!("{} is {}", f, v.get_string_value());
                }
                None => eprintln!("{} is not present", f),
            });
        ["iat", "nbf", "exp"]
            .iter()
            .for_each(|&f| match jwt_payload_fields.get(f) {
                Some(v) => {
                    assert!(v.has_number_value());
                    eprintln!("{} is {}", f, v.get_number_value());
                }
                None => eprintln!("{} is not present", f),
            });
        // aud can be a string or a list of strings, or empty _iff_ azp is present
        let aud = match jwt_payload_fields.get("aud") {
            Some(v) => {
                if v.has_string_value() {
                    vec![v.get_string_value()]
                } else {
                    assert!(v.has_list_value());
                    let v = v.get_list_value();
                    v.values
                        .iter()
                        .map(|v| {
                            // all items in the list of values should be strings
                            assert!(v.has_string_value());
                            v.get_string_value()
                        })
                        .collect::<Vec<_>>()
                }
            }
            None => {
                vec![]
            }
        };
        eprintln!("aud is {:?}", aud);
        let azp = match jwt_payload_fields.get("azp") {
            Some(v) => {
                assert!(v.has_string_value());
                let v = v.get_string_value();
                eprintln!("azp is {}", v);
                v
            }
            None => {
                assert!(!aud.is_empty(), "both aud and azp cannot be empty");
                ""
            }
        };
        let app_id = if azp.is_empty() { aud[0] } else { azp };
        eprintln!("app_id is {}", app_id);
        assert!(!app_id.is_empty());
        let jwt_signature = base64::decode_config(jwt_parts[2], base64::URL_SAFE);
        assert!(jwt_signature.is_ok());
        let jwt_signature = jwt_signature.unwrap();
        let jwt_signature_s = String::from_utf8_lossy(jwt_signature.as_slice());
        // JWT signature is not JSON and should not be loaded into a protobuf struct
        eprintln!("JWT signature: {}", jwt_signature_s);
    }

    #[test]
    fn jwt() {
        let jwt_json = r#"{
            "exp": 1614620927,
            "iat": 1614620867,
            "auth_time": 1614620866,
            "jti": "322dace5-75ee-430b-9cad-2aa04c2b3527",
            "iss": "https://keycloak:8443/auth/realms/master",
            "aud": "test",
            "sub": "a7152af4-6cb4-48db-b28f-cb4e7af11b59",
            "typ": "ID",
            "azp": "test",
            "session_state": "51ff0423-83d3-4424-b286-15fb9722f845",
            "at_hash": "nFsXrDToP2hyB-nSnpzdhw",
            "acr": "1",
            "email_verified": false,
            "preferred_username": "admin"
        }"#;
        let jwt = serde_json::from_str::<JWT>(jwt_json);
        match jwt {
            Ok(_) => (),
            Err(ref e) => eprintln!("{}", crate::util::serde_json_error_to_string(e, jwt_json)),
        };
        assert!(jwt.is_ok());
        let jwt = jwt.unwrap();
        eprintln!(
            "JWT as JSON is\n{}",
            serde_json::to_string_pretty(&jwt).unwrap()
        );
        //{
        //    use protobuf::Message;
        //    let jj = protobuf::Message::write_to_bytes(&jwt);
        //}
        //protobuf::json::print_to_string(message)
        assert_eq!(1, 1);
    }
}
