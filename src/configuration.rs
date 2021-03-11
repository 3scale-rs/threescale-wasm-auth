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
    locations: Vec<Location>,
    kind: ApplicationKind,
    keys: Vec<K>,
    #[serde(flatten)]
    other: HashMap<String, serde_json::Value>,
}

impl<K> Parameter<K> {
    pub fn locations(&self) -> &Vec<Location> {
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
    valid_apps: Option<Vec<String>>,
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

    pub fn valid_apps(&self) -> Option<&Vec<String>> {
        self.valid_apps.as_ref()
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
    system: Option<System>,
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
    pub fn system(&self) -> Option<&System> {
        self.system.as_ref()
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
    use std::io::Write;

    use protobuf::Message;

    use self::fixtures::*;
    use super::*;

    mod fixtures {
        pub const JWT: &str = "eyJhbGciOiJSUzI1NiIsInR5cCIgOiAiSldUIiwia2lkIiA6ICI4VFJ3cHZKb1M3N3A0MF9YVzhzSl9jaHhyNDFzN2l6U05LUEoyaHhXcl9RIn0.eyJleHAiOjE2MTQ3NjgxNTYsImlhdCI6MTYxNDc2ODA5NiwiYXV0aF90aW1lIjoxNjE0NzY4MDk1LCJqdGkiOiJmMjA0ZDBkOC04MTAwLTQ1ZmMtOGUxNS1kMDJiNjdkYWFjODQiLCJpc3MiOiJodHRwczovL2tleWNsb2FrOjg0NDMvYXV0aC9yZWFsbXMvbWFzdGVyIiwiYXVkIjoidGVzdCIsInN1YiI6IjJlMTc2Yjk2LTJmZDEtNGM2OS1hMTYyLTFmOTY0YjkwNmM0ZCIsInR5cCI6IklEIiwiYXpwIjoidGVzdCIsInNlc3Npb25fc3RhdGUiOiI4Y2UxOGJiOC1jMGU0LTRiNTktOGMwNS04YTJlMmRlOWU1ODgiLCJhdF9oYXNoIjoiRXpwNjVFYkZnTFA1RWY0SEM4MzNqdyIsImFjciI6IjEiLCJlbWFpbF92ZXJpZmllZCI6ZmFsc2UsInByZWZlcnJlZF91c2VybmFtZSI6ImFkbWluIn0.dcaPCgYg92Z6CG4lxUFiaw2YIprQmAuH9sJ8d1RogWZT9AUKHMLYlBVf4Rnx6V9NZj6fumvXQhF3bzw8gwK6kIM3L5_lcQwyFuz6Ss7uG3bmTomXbNgo9kkw1TLAgrrqj6U0GDjIpvNxjTmy5UqX0fGHlMmasUPIm0MbobAfR1VKSaokTQO42MdnvMD9XCmq0-ty-u8B1XQaQPPiy2eNI3zGbkuPyMR1pCnjhd-UXhsWMFmQW9KQWZQZqX164AQnIyjf-KuLiFxd8sZZmpZe8bQmbC8K8exjPvhGJprW3qus9dThFweoSbnB8QG08Sc-OM4tLe0GpSgJl1fzqU5afg";
        pub const JWT_PAYLOAD: &str = "eyJleHAiOjE2MTQ3NjgxNTYsImlhdCI6MTYxNDc2ODA5NiwiYXV0aF90aW1lIjoxNjE0NzY4MDk1LCJqdGkiOiJmMjA0ZDBkOC04MTAwLTQ1ZmMtOGUxNS1kMDJiNjdkYWFjODQiLCJpc3MiOiJodHRwczovL2tleWNsb2FrOjg0NDMvYXV0aC9yZWFsbXMvbWFzdGVyIiwiYXVkIjoidGVzdCIsInN1YiI6IjJlMTc2Yjk2LTJmZDEtNGM2OS1hMTYyLTFmOTY0YjkwNmM0ZCIsInR5cCI6IklEIiwiYXpwIjoidGVzdCIsInNlc3Npb25fc3RhdGUiOiI4Y2UxOGJiOC1jMGU0LTRiNTktOGMwNS04YTJlMmRlOWU1ODgiLCJhdF9oYXNoIjoiRXpwNjVFYkZnTFA1RWY0SEM4MzNqdyIsImFjciI6IjEiLCJlbWFpbF92ZXJpZmllZCI6ZmFsc2UsInByZWZlcnJlZF91c2VybmFtZSI6ImFkbWluIn0";
        pub const JWT_JSON: &str = r#"{
            "exp": 1614768156,
            "iat": 1614768096,
            "auth_time": 1614768095,
            "jti": "f204d0d8-8100-45fc-8e15-d02b67daac84",
            "iss": "https://keycloak:8443/auth/realms/master",
            "aud": "test",
            "sub": "2e176b96-2fd1-4c69-a162-1f964b906c4d",
            "typ": "ID",
            "azp": "test",
            "session_state": "8ce18bb8-c0e4-4b59-8c05-8a2e2de9e588",
            "at_hash": "Ezp65EbFgLP5Ef4HC833jw",
            "acr": "1",
            "email_verified": false,
            "preferred_username": "admin"
        }"#;
        // as output by jwt_authn
        pub const JWT_PAYLOAD_PB: &[u8] = &[
            0xe, 0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x0, 0x28, 0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x0, 0x8,
            0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x12, 0x0, 0x0, 0x0, 0x5, 0x0,
            0x0, 0x0, 0x3, 0x0, 0x0, 0x0, 0x2, 0x0, 0x0, 0x0, 0x7, 0x0, 0x0, 0x0, 0x16, 0x0, 0x0,
            0x0, 0x3, 0x0, 0x0, 0x0, 0x8, 0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x0, 0x4, 0x0, 0x0, 0x0,
            0x3, 0x0, 0x0, 0x0, 0x24, 0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x0, 0x24, 0x0, 0x0, 0x0, 0x9,
            0x0, 0x0, 0x0, 0x8, 0x0, 0x0, 0x0, 0xe, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x3, 0x0,
            0x0, 0x0, 0x4, 0x0, 0x0, 0x0, 0xd, 0x0, 0x0, 0x0, 0x24, 0x0, 0x0, 0x0, 0x69, 0x73,
            0x73, 0x0, 0x68, 0x74, 0x74, 0x70, 0x73, 0x3a, 0x2f, 0x2f, 0x6b, 0x65, 0x79, 0x63,
            0x6c, 0x6f, 0x61, 0x6b, 0x3a, 0x38, 0x34, 0x34, 0x33, 0x2f, 0x61, 0x75, 0x74, 0x68,
            0x2f, 0x72, 0x65, 0x61, 0x6c, 0x6d, 0x73, 0x2f, 0x6d, 0x61, 0x73, 0x74, 0x65, 0x72,
            0x0, 0x69, 0x61, 0x74, 0x0, 0x0, 0x0, 0x0, 0xf8, 0xd9, 0xf, 0xd8, 0x41, 0x0, 0x61,
            0x63, 0x72, 0x0, 0x31, 0x0, 0x70, 0x72, 0x65, 0x66, 0x65, 0x72, 0x72, 0x65, 0x64, 0x5f,
            0x75, 0x73, 0x65, 0x72, 0x6e, 0x61, 0x6d, 0x65, 0x0, 0x61, 0x64, 0x6d, 0x69, 0x6e, 0x0,
            0x74, 0x79, 0x70, 0x0, 0x49, 0x44, 0x0, 0x61, 0x74, 0x5f, 0x68, 0x61, 0x73, 0x68, 0x0,
            0x45, 0x7a, 0x70, 0x36, 0x35, 0x45, 0x62, 0x46, 0x67, 0x4c, 0x50, 0x35, 0x45, 0x66,
            0x34, 0x48, 0x43, 0x38, 0x33, 0x33, 0x6a, 0x77, 0x0, 0x65, 0x78, 0x70, 0x0, 0x0, 0x0,
            0x0, 0x7, 0xda, 0xf, 0xd8, 0x41, 0x0, 0x61, 0x75, 0x64, 0x0, 0x74, 0x65, 0x73, 0x74,
            0x0, 0x73, 0x75, 0x62, 0x0, 0x32, 0x65, 0x31, 0x37, 0x36, 0x62, 0x39, 0x36, 0x2d, 0x32,
            0x66, 0x64, 0x31, 0x2d, 0x34, 0x63, 0x36, 0x39, 0x2d, 0x61, 0x31, 0x36, 0x32, 0x2d,
            0x31, 0x66, 0x39, 0x36, 0x34, 0x62, 0x39, 0x30, 0x36, 0x63, 0x34, 0x64, 0x0, 0x6a,
            0x74, 0x69, 0x0, 0x66, 0x32, 0x30, 0x34, 0x64, 0x30, 0x64, 0x38, 0x2d, 0x38, 0x31,
            0x30, 0x30, 0x2d, 0x34, 0x35, 0x66, 0x63, 0x2d, 0x38, 0x65, 0x31, 0x35, 0x2d, 0x64,
            0x30, 0x32, 0x62, 0x36, 0x37, 0x64, 0x61, 0x61, 0x63, 0x38, 0x34, 0x0, 0x61, 0x75,
            0x74, 0x68, 0x5f, 0x74, 0x69, 0x6d, 0x65, 0x0, 0x0, 0x0, 0xc0, 0xf7, 0xd9, 0xf, 0xd8,
            0x41, 0x0, 0x65, 0x6d, 0x61, 0x69, 0x6c, 0x5f, 0x76, 0x65, 0x72, 0x69, 0x66, 0x69,
            0x65, 0x64, 0x0, 0x0, 0x0, 0x61, 0x7a, 0x70, 0x0, 0x74, 0x65, 0x73, 0x74, 0x0, 0x73,
            0x65, 0x73, 0x73, 0x69, 0x6f, 0x6e, 0x5f, 0x73, 0x74, 0x61, 0x74, 0x65, 0x0, 0x38,
            0x63, 0x65, 0x31, 0x38, 0x62, 0x62, 0x38, 0x2d, 0x63, 0x30, 0x65, 0x34, 0x2d, 0x34,
            0x62, 0x35, 0x39, 0x2d, 0x38, 0x63, 0x30, 0x35, 0x2d, 0x38, 0x61, 0x32, 0x65, 0x32,
            0x64, 0x65, 0x39, 0x65, 0x35, 0x38, 0x38, 0x0,
        ];
        pub const EXAMPLE_METADATA: &[u8] = &[
            0xa, 0xcf, 0x3, 0xa, 0x1c, 0x65, 0x6e, 0x76, 0x6f, 0x79, 0x2e, 0x66, 0x69, 0x6c, 0x74,
            0x65, 0x72, 0x73, 0x2e, 0x68, 0x74, 0x74, 0x70, 0x2e, 0x6a, 0x77, 0x74, 0x5f, 0x61,
            0x75, 0x74, 0x68, 0x6e, 0x12, 0xae, 0x3, 0xa, 0xab, 0x3, 0xa, 0xc, 0x76, 0x65, 0x72,
            0x69, 0x66, 0x69, 0x65, 0x64, 0x5f, 0x6a, 0x77, 0x74, 0x12, 0x9a, 0x3, 0x2a, 0x97, 0x3,
            0xa, 0xd, 0xa, 0x3, 0x61, 0x7a, 0x70, 0x12, 0x6, 0x1a, 0x4, 0x74, 0x65, 0x73, 0x74,
            0xa, 0x37, 0xa, 0xd, 0x73, 0x65, 0x73, 0x73, 0x69, 0x6f, 0x6e, 0x5f, 0x73, 0x74, 0x61,
            0x74, 0x65, 0x12, 0x26, 0x1a, 0x24, 0x64, 0x61, 0x35, 0x61, 0x66, 0x33, 0x39, 0x66,
            0x2d, 0x66, 0x39, 0x39, 0x35, 0x2d, 0x34, 0x32, 0x63, 0x30, 0x2d, 0x61, 0x31, 0x31,
            0x37, 0x2d, 0x63, 0x36, 0x37, 0x32, 0x61, 0x32, 0x31, 0x39, 0x64, 0x61, 0x36, 0x39,
            0xa, 0x31, 0xa, 0x3, 0x69, 0x73, 0x73, 0x12, 0x2a, 0x1a, 0x28, 0x68, 0x74, 0x74, 0x70,
            0x73, 0x3a, 0x2f, 0x2f, 0x6b, 0x65, 0x79, 0x63, 0x6c, 0x6f, 0x61, 0x6b, 0x3a, 0x38,
            0x34, 0x34, 0x33, 0x2f, 0x61, 0x75, 0x74, 0x68, 0x2f, 0x72, 0x65, 0x61, 0x6c, 0x6d,
            0x73, 0x2f, 0x6d, 0x61, 0x73, 0x74, 0x65, 0x72, 0xa, 0x1d, 0xa, 0x12, 0x70, 0x72, 0x65,
            0x66, 0x65, 0x72, 0x72, 0x65, 0x64, 0x5f, 0x75, 0x73, 0x65, 0x72, 0x6e, 0x61, 0x6d,
            0x65, 0x12, 0x7, 0x1a, 0x5, 0x61, 0x64, 0x6d, 0x69, 0x6e, 0xa, 0xa, 0xa, 0x3, 0x61,
            0x63, 0x72, 0x12, 0x3, 0x1a, 0x1, 0x31, 0xa, 0x10, 0xa, 0x3, 0x69, 0x61, 0x74, 0x12,
            0x9, 0x11, 0x0, 0x0, 0x80, 0x55, 0xec, 0xf, 0xd8, 0x41, 0xa, 0xb, 0xa, 0x3, 0x74, 0x79,
            0x70, 0x12, 0x4, 0x1a, 0x2, 0x49, 0x44, 0xa, 0x23, 0xa, 0x7, 0x61, 0x74, 0x5f, 0x68,
            0x61, 0x73, 0x68, 0x12, 0x18, 0x1a, 0x16, 0x47, 0x4e, 0x44, 0x5f, 0x5a, 0x50, 0x33,
            0x45, 0x59, 0x2d, 0x61, 0x6d, 0x47, 0x56, 0x37, 0x49, 0x4c, 0x45, 0x45, 0x77, 0x54,
            0x77, 0xa, 0x10, 0xa, 0x3, 0x65, 0x78, 0x70, 0x12, 0x9, 0x11, 0x0, 0x0, 0x80, 0x64,
            0xec, 0xf, 0xd8, 0x41, 0xa, 0xd, 0xa, 0x3, 0x61, 0x75, 0x64, 0x12, 0x6, 0x1a, 0x4,
            0x74, 0x65, 0x73, 0x74, 0xa, 0x2d, 0xa, 0x3, 0x73, 0x75, 0x62, 0x12, 0x26, 0x1a, 0x24,
            0x32, 0x65, 0x31, 0x37, 0x36, 0x62, 0x39, 0x36, 0x2d, 0x32, 0x66, 0x64, 0x31, 0x2d,
            0x34, 0x63, 0x36, 0x39, 0x2d, 0x61, 0x31, 0x36, 0x32, 0x2d, 0x31, 0x66, 0x39, 0x36,
            0x34, 0x62, 0x39, 0x30, 0x36, 0x63, 0x34, 0x64, 0xa, 0x2d, 0xa, 0x3, 0x6a, 0x74, 0x69,
            0x12, 0x26, 0x1a, 0x24, 0x32, 0x33, 0x38, 0x32, 0x64, 0x65, 0x65, 0x36, 0x2d, 0x39,
            0x37, 0x61, 0x32, 0x2d, 0x34, 0x34, 0x66, 0x63, 0x2d, 0x62, 0x35, 0x61, 0x64, 0x2d,
            0x33, 0x34, 0x61, 0x35, 0x31, 0x30, 0x66, 0x39, 0x36, 0x66, 0x34, 0x30, 0xa, 0x16, 0xa,
            0x9, 0x61, 0x75, 0x74, 0x68, 0x5f, 0x74, 0x69, 0x6d, 0x65, 0x12, 0x9, 0x11, 0x0, 0x0,
            0x40, 0x55, 0xec, 0xf, 0xd8, 0x41, 0xa, 0x14, 0xa, 0xe, 0x65, 0x6d, 0x61, 0x69, 0x6c,
            0x5f, 0x76, 0x65, 0x72, 0x69, 0x66, 0x69, 0x65, 0x64, 0x12, 0x2, 0x20, 0x0,
        ];
        // XXX this one does not really parse
        pub const EXAMPLE_METADATA_FILTER_METADATA: &[u8] = &[
            0x1, 0x0, 0x0, 0x0, 0x1c, 0x0, 0x0, 0x0, 0xd5, 0x1, 0x0, 0x0, 0x65, 0x6e, 0x76, 0x6f,
            0x79, 0x2e, 0x66, 0x69, 0x6c, 0x74, 0x65, 0x72, 0x73, 0x2e, 0x68, 0x74, 0x74, 0x70,
            0x2e, 0x6a, 0x77, 0x74, 0x5f, 0x61, 0x75, 0x74, 0x68, 0x6e, 0x0, 0x1, 0x0, 0x0, 0x0,
            0xc, 0x0, 0x0, 0x0, 0xbb, 0x1, 0x0, 0x0, 0x76, 0x65, 0x72, 0x69, 0x66, 0x69, 0x65,
            0x64, 0x5f, 0x6a, 0x77, 0x74, 0x0, 0xe, 0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x0, 0x1, 0x0,
            0x0, 0x0, 0x12, 0x0, 0x0, 0x0, 0x5, 0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x0, 0x8, 0x0, 0x0,
            0x0, 0x3, 0x0, 0x0, 0x0, 0x2, 0x0, 0x0, 0x0, 0x7, 0x0, 0x0, 0x0, 0x16, 0x0, 0x0, 0x0,
            0x3, 0x0, 0x0, 0x0, 0x8, 0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x0, 0x4, 0x0, 0x0, 0x0, 0x3,
            0x0, 0x0, 0x0, 0x24, 0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x0, 0x24, 0x0, 0x0, 0x0, 0x9, 0x0,
            0x0, 0x0, 0x8, 0x0, 0x0, 0x0, 0xe, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x3, 0x0, 0x0,
            0x0, 0x4, 0x0, 0x0, 0x0, 0xd, 0x0, 0x0, 0x0, 0x24, 0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x0,
            0x28, 0x0, 0x0, 0x0, 0x61, 0x63, 0x72, 0x0, 0x31, 0x0, 0x70, 0x72, 0x65, 0x66, 0x65,
            0x72, 0x72, 0x65, 0x64, 0x5f, 0x75, 0x73, 0x65, 0x72, 0x6e, 0x61, 0x6d, 0x65, 0x0,
            0x61, 0x64, 0x6d, 0x69, 0x6e, 0x0, 0x69, 0x61, 0x74, 0x0, 0x0, 0x0, 0x80, 0x55, 0xec,
            0xf, 0xd8, 0x41, 0x0, 0x74, 0x79, 0x70, 0x0, 0x49, 0x44, 0x0, 0x61, 0x74, 0x5f, 0x68,
            0x61, 0x73, 0x68, 0x0, 0x47, 0x4e, 0x44, 0x5f, 0x5a, 0x50, 0x33, 0x45, 0x59, 0x2d,
            0x61, 0x6d, 0x47, 0x56, 0x37, 0x49, 0x4c, 0x45, 0x45, 0x77, 0x54, 0x77, 0x0, 0x65,
            0x78, 0x70, 0x0, 0x0, 0x0, 0x80, 0x64, 0xec, 0xf, 0xd8, 0x41, 0x0, 0x61, 0x75, 0x64,
            0x0, 0x74, 0x65, 0x73, 0x74, 0x0, 0x73, 0x75, 0x62, 0x0, 0x32, 0x65, 0x31, 0x37, 0x36,
            0x62, 0x39, 0x36, 0x2d, 0x32, 0x66, 0x64, 0x31, 0x2d, 0x34, 0x63, 0x36, 0x39, 0x2d,
            0x61, 0x31, 0x36, 0x32, 0x2d, 0x31, 0x66, 0x39, 0x36, 0x34, 0x62, 0x39, 0x30, 0x36,
            0x63, 0x34, 0x64, 0x0, 0x6a, 0x74, 0x69, 0x0, 0x32, 0x33, 0x38, 0x32, 0x64, 0x65, 0x65,
            0x36, 0x2d, 0x39, 0x37, 0x61, 0x32, 0x2d, 0x34, 0x34, 0x66, 0x63, 0x2d, 0x62, 0x35,
            0x61, 0x64, 0x2d, 0x33, 0x34, 0x61, 0x35, 0x31, 0x30, 0x66, 0x39, 0x36, 0x66, 0x34,
            0x30, 0x0, 0x61, 0x75, 0x74, 0x68, 0x5f, 0x74, 0x69, 0x6d, 0x65, 0x0, 0x0, 0x0, 0x40,
            0x55, 0xec, 0xf, 0xd8, 0x41, 0x0, 0x65, 0x6d, 0x61, 0x69, 0x6c, 0x5f, 0x76, 0x65, 0x72,
            0x69, 0x66, 0x69, 0x65, 0x64, 0x0, 0x0, 0x0, 0x61, 0x7a, 0x70, 0x0, 0x74, 0x65, 0x73,
            0x74, 0x0, 0x73, 0x65, 0x73, 0x73, 0x69, 0x6f, 0x6e, 0x5f, 0x73, 0x74, 0x61, 0x74,
            0x65, 0x0, 0x64, 0x61, 0x35, 0x61, 0x66, 0x33, 0x39, 0x66, 0x2d, 0x66, 0x39, 0x39,
            0x35, 0x2d, 0x34, 0x32, 0x63, 0x30, 0x2d, 0x61, 0x31, 0x31, 0x37, 0x2d, 0x63, 0x36,
            0x37, 0x32, 0x61, 0x32, 0x31, 0x39, 0x64, 0x61, 0x36, 0x39, 0x0, 0x69, 0x73, 0x73, 0x0,
            0x68, 0x74, 0x74, 0x70, 0x73, 0x3a, 0x2f, 0x2f, 0x6b, 0x65, 0x79, 0x63, 0x6c, 0x6f,
            0x61, 0x6b, 0x3a, 0x38, 0x34, 0x34, 0x33, 0x2f, 0x61, 0x75, 0x74, 0x68, 0x2f, 0x72,
            0x65, 0x61, 0x6c, 0x6d, 0x73, 0x2f, 0x6d, 0x61, 0x73, 0x74, 0x65, 0x72, 0x0, 0x0, 0x0,
        ];
        pub const EXAMPLE_METADATA_FILTER_METADATA_ENVOY_JWT_AUTHN: &[u8] = &[
            0x1, 0x0, 0x0, 0x0, 0xc, 0x0, 0x0, 0x0, 0xbb, 0x1, 0x0, 0x0, 0x76, 0x65, 0x72, 0x69,
            0x66, 0x69, 0x65, 0x64, 0x5f, 0x6a, 0x77, 0x74, 0x0, 0xe, 0x0, 0x0, 0x0, 0x3, 0x0, 0x0,
            0x0, 0x1, 0x0, 0x0, 0x0, 0x12, 0x0, 0x0, 0x0, 0x5, 0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x0,
            0x8, 0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x0, 0x2, 0x0, 0x0, 0x0, 0x7, 0x0, 0x0, 0x0, 0x16,
            0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x0, 0x8, 0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x0, 0x4, 0x0,
            0x0, 0x0, 0x3, 0x0, 0x0, 0x0, 0x24, 0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x0, 0x24, 0x0, 0x0,
            0x0, 0x9, 0x0, 0x0, 0x0, 0x8, 0x0, 0x0, 0x0, 0xe, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0,
            0x3, 0x0, 0x0, 0x0, 0x4, 0x0, 0x0, 0x0, 0xd, 0x0, 0x0, 0x0, 0x24, 0x0, 0x0, 0x0, 0x3,
            0x0, 0x0, 0x0, 0x28, 0x0, 0x0, 0x0, 0x61, 0x63, 0x72, 0x0, 0x31, 0x0, 0x70, 0x72, 0x65,
            0x66, 0x65, 0x72, 0x72, 0x65, 0x64, 0x5f, 0x75, 0x73, 0x65, 0x72, 0x6e, 0x61, 0x6d,
            0x65, 0x0, 0x61, 0x64, 0x6d, 0x69, 0x6e, 0x0, 0x69, 0x61, 0x74, 0x0, 0x0, 0x0, 0x80,
            0x55, 0xec, 0xf, 0xd8, 0x41, 0x0, 0x74, 0x79, 0x70, 0x0, 0x49, 0x44, 0x0, 0x61, 0x74,
            0x5f, 0x68, 0x61, 0x73, 0x68, 0x0, 0x47, 0x4e, 0x44, 0x5f, 0x5a, 0x50, 0x33, 0x45,
            0x59, 0x2d, 0x61, 0x6d, 0x47, 0x56, 0x37, 0x49, 0x4c, 0x45, 0x45, 0x77, 0x54, 0x77,
            0x0, 0x65, 0x78, 0x70, 0x0, 0x0, 0x0, 0x80, 0x64, 0xec, 0xf, 0xd8, 0x41, 0x0, 0x61,
            0x75, 0x64, 0x0, 0x74, 0x65, 0x73, 0x74, 0x0, 0x73, 0x75, 0x62, 0x0, 0x32, 0x65, 0x31,
            0x37, 0x36, 0x62, 0x39, 0x36, 0x2d, 0x32, 0x66, 0x64, 0x31, 0x2d, 0x34, 0x63, 0x36,
            0x39, 0x2d, 0x61, 0x31, 0x36, 0x32, 0x2d, 0x31, 0x66, 0x39, 0x36, 0x34, 0x62, 0x39,
            0x30, 0x36, 0x63, 0x34, 0x64, 0x0, 0x6a, 0x74, 0x69, 0x0, 0x32, 0x33, 0x38, 0x32, 0x64,
            0x65, 0x65, 0x36, 0x2d, 0x39, 0x37, 0x61, 0x32, 0x2d, 0x34, 0x34, 0x66, 0x63, 0x2d,
            0x62, 0x35, 0x61, 0x64, 0x2d, 0x33, 0x34, 0x61, 0x35, 0x31, 0x30, 0x66, 0x39, 0x36,
            0x66, 0x34, 0x30, 0x0, 0x61, 0x75, 0x74, 0x68, 0x5f, 0x74, 0x69, 0x6d, 0x65, 0x0, 0x0,
            0x0, 0x40, 0x55, 0xec, 0xf, 0xd8, 0x41, 0x0, 0x65, 0x6d, 0x61, 0x69, 0x6c, 0x5f, 0x76,
            0x65, 0x72, 0x69, 0x66, 0x69, 0x65, 0x64, 0x0, 0x0, 0x0, 0x61, 0x7a, 0x70, 0x0, 0x74,
            0x65, 0x73, 0x74, 0x0, 0x73, 0x65, 0x73, 0x73, 0x69, 0x6f, 0x6e, 0x5f, 0x73, 0x74,
            0x61, 0x74, 0x65, 0x0, 0x64, 0x61, 0x35, 0x61, 0x66, 0x33, 0x39, 0x66, 0x2d, 0x66,
            0x39, 0x39, 0x35, 0x2d, 0x34, 0x32, 0x63, 0x30, 0x2d, 0x61, 0x31, 0x31, 0x37, 0x2d,
            0x63, 0x36, 0x37, 0x32, 0x61, 0x32, 0x31, 0x39, 0x64, 0x61, 0x36, 0x39, 0x0, 0x69,
            0x73, 0x73, 0x0, 0x68, 0x74, 0x74, 0x70, 0x73, 0x3a, 0x2f, 0x2f, 0x6b, 0x65, 0x79,
            0x63, 0x6c, 0x6f, 0x61, 0x6b, 0x3a, 0x38, 0x34, 0x34, 0x33, 0x2f, 0x61, 0x75, 0x74,
            0x68, 0x2f, 0x72, 0x65, 0x61, 0x6c, 0x6d, 0x73, 0x2f, 0x6d, 0x61, 0x73, 0x74, 0x65,
            0x72, 0x0, 0x0,
        ];
        pub const EXAMPLE_METADATA_FILTER_METADATA_ENVOY_JWT_AUTHN_VERIFIED_JWT: &[u8] = &[
            0xe, 0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x12, 0x0, 0x0, 0x0, 0x5,
            0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x0, 0x8, 0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x0, 0x2, 0x0,
            0x0, 0x0, 0x7, 0x0, 0x0, 0x0, 0x16, 0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x0, 0x8, 0x0, 0x0,
            0x0, 0x3, 0x0, 0x0, 0x0, 0x4, 0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x0, 0x24, 0x0, 0x0, 0x0,
            0x3, 0x0, 0x0, 0x0, 0x24, 0x0, 0x0, 0x0, 0x9, 0x0, 0x0, 0x0, 0x8, 0x0, 0x0, 0x0, 0xe,
            0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x0, 0x4, 0x0, 0x0, 0x0, 0xd, 0x0,
            0x0, 0x0, 0x24, 0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x0, 0x28, 0x0, 0x0, 0x0, 0x61, 0x63,
            0x72, 0x0, 0x31, 0x0, 0x70, 0x72, 0x65, 0x66, 0x65, 0x72, 0x72, 0x65, 0x64, 0x5f, 0x75,
            0x73, 0x65, 0x72, 0x6e, 0x61, 0x6d, 0x65, 0x0, 0x61, 0x64, 0x6d, 0x69, 0x6e, 0x0, 0x69,
            0x61, 0x74, 0x0, 0x0, 0x0, 0x80, 0x55, 0xec, 0xf, 0xd8, 0x41, 0x0, 0x74, 0x79, 0x70,
            0x0, 0x49, 0x44, 0x0, 0x61, 0x74, 0x5f, 0x68, 0x61, 0x73, 0x68, 0x0, 0x47, 0x4e, 0x44,
            0x5f, 0x5a, 0x50, 0x33, 0x45, 0x59, 0x2d, 0x61, 0x6d, 0x47, 0x56, 0x37, 0x49, 0x4c,
            0x45, 0x45, 0x77, 0x54, 0x77, 0x0, 0x65, 0x78, 0x70, 0x0, 0x0, 0x0, 0x80, 0x64, 0xec,
            0xf, 0xd8, 0x41, 0x0, 0x61, 0x75, 0x64, 0x0, 0x74, 0x65, 0x73, 0x74, 0x0, 0x73, 0x75,
            0x62, 0x0, 0x32, 0x65, 0x31, 0x37, 0x36, 0x62, 0x39, 0x36, 0x2d, 0x32, 0x66, 0x64,
            0x31, 0x2d, 0x34, 0x63, 0x36, 0x39, 0x2d, 0x61, 0x31, 0x36, 0x32, 0x2d, 0x31, 0x66,
            0x39, 0x36, 0x34, 0x62, 0x39, 0x30, 0x36, 0x63, 0x34, 0x64, 0x0, 0x6a, 0x74, 0x69, 0x0,
            0x32, 0x33, 0x38, 0x32, 0x64, 0x65, 0x65, 0x36, 0x2d, 0x39, 0x37, 0x61, 0x32, 0x2d,
            0x34, 0x34, 0x66, 0x63, 0x2d, 0x62, 0x35, 0x61, 0x64, 0x2d, 0x33, 0x34, 0x61, 0x35,
            0x31, 0x30, 0x66, 0x39, 0x36, 0x66, 0x34, 0x30, 0x0, 0x61, 0x75, 0x74, 0x68, 0x5f,
            0x74, 0x69, 0x6d, 0x65, 0x0, 0x0, 0x0, 0x40, 0x55, 0xec, 0xf, 0xd8, 0x41, 0x0, 0x65,
            0x6d, 0x61, 0x69, 0x6c, 0x5f, 0x76, 0x65, 0x72, 0x69, 0x66, 0x69, 0x65, 0x64, 0x0, 0x0,
            0x0, 0x61, 0x7a, 0x70, 0x0, 0x74, 0x65, 0x73, 0x74, 0x0, 0x73, 0x65, 0x73, 0x73, 0x69,
            0x6f, 0x6e, 0x5f, 0x73, 0x74, 0x61, 0x74, 0x65, 0x0, 0x64, 0x61, 0x35, 0x61, 0x66,
            0x33, 0x39, 0x66, 0x2d, 0x66, 0x39, 0x39, 0x35, 0x2d, 0x34, 0x32, 0x63, 0x30, 0x2d,
            0x61, 0x31, 0x31, 0x37, 0x2d, 0x63, 0x36, 0x37, 0x32, 0x61, 0x32, 0x31, 0x39, 0x64,
            0x61, 0x36, 0x39, 0x0, 0x69, 0x73, 0x73, 0x0, 0x68, 0x74, 0x74, 0x70, 0x73, 0x3a, 0x2f,
            0x2f, 0x6b, 0x65, 0x79, 0x63, 0x6c, 0x6f, 0x61, 0x6b, 0x3a, 0x38, 0x34, 0x34, 0x33,
            0x2f, 0x61, 0x75, 0x74, 0x68, 0x2f, 0x72, 0x65, 0x61, 0x6c, 0x6d, 0x73, 0x2f, 0x6d,
            0x61, 0x73, 0x74, 0x65, 0x72, 0x0,
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
                      "header": {
                          "keys": ["x-api-key"]
                      },
                      "query_string": {
                          "keys": ["x-api-key"]
                      }
                    ]
                  },
                  {
                    "kind": "oidc",
                    "keys": ["aud", "azp"],
                    "locations": [
                        "property": {
                            "path": ["metadata", "filter_metadata", "envoy.filters.http.jwt_authn"],
                            "format": "string",
                            "keys": ["azp", "aud"]
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
            system: Some(System {
                name: Some("system-name".into()),
                upstream: Upstream {
                    name: "outbound|443||multitenant.3scale.net".into(),
                    url: "https://istiodevel-admin.3scale.net".parse().unwrap(),
                    timeout: core::time::Duration::from_millis(5000),
                },
                token: "atoken".into(),
            }),
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
                valid_apps: None,
                authorities: vec!["0.0.0.0:8080".into(), "0.0.0.0:8443".into()],
                credentials: vec![Parameter::<String> {
                    other: HashMap::new(),
                    kind: ApplicationKind::OIDC,
                    keys: vec!["azp".into(), "aud".into(), "x-jwt-payload".into()],
                    locations: vec![
                        Location::Header {
                            keys: vec!["abc".into()],
                            ops: Some(vec![
                                Operation::Decode(Decode::Base64URLDecode),
                                Operation::Decode(Decode::JsonValue),
                            ]),
                        },
                        Location::Property {
                            path: vec![
                                "metadata".into(),
                                "filter_metadata".into(),
                                "envoy.filters.http.jwt_authn".into(),
                            ],
                            format: Format::Pairs,
                            ops: Some(vec![
                                Operation::Lookup {
                                    input: Format::Pairs,
                                    kind: LookupType::Key("verified_jwt".into()),
                                    output: Format::Pairs,
                                },
                                // these two together don't make sense in this case, but this is a demo
                                Operation::Lookup {
                                    input: Format::Pairs,
                                    kind: LookupType::Position(0),
                                    output: Format::Pairs,
                                },
                            ]),
                            keys: vec!["azp".into(), "aud".into()],
                        },
                        Location::Property {
                            path: vec!["metadata".into()],
                            format: Format::ProtobufStruct,
                            ops: Some(vec![
                                Operation::Lookup {
                                    input: Format::ProtobufStruct,
                                    output: Format::ProtobufStruct,
                                    kind: LookupType::Key("filter_metadata".into()),
                                },
                                Operation::Lookup {
                                    input: Format::ProtobufStruct,
                                    output: Format::ProtobufStruct,
                                    kind: LookupType::Key("envoy.filters.http.jwt_authn".into()),
                                },
                                Operation::Lookup {
                                    input: Format::ProtobufStruct,
                                    output: Format::ProtobufStruct,
                                    kind: LookupType::Position(0),
                                },
                            ]),
                            keys: vec!["azp".into(), "aud".into()],
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
        let jwt = JWT;
        let jwt_parts = jwt.split('.').collect::<Vec<_>>();
        assert_eq!(jwt_parts.len(), 3);
        let jwt_header = base64::decode_config(jwt_parts[0], base64::URL_SAFE);
        assert!(jwt_header.is_ok());
        let jwt_header = jwt_header.unwrap();
        // generate message with something like prost::json::StringToMessage(&jwt_first)
        let jwt_header_s = unsafe { String::from_utf8_unchecked(jwt_header) };
        let jwt_header_pb = protobuf::json::parse_from_str::<protobuf::well_known_types::Struct>(
            jwt_header_s.as_str(),
        );
        assert!(jwt_header_pb.is_ok());
        let jwt_header_pb = jwt_header_pb.unwrap();
        let jwt_header_fields = &jwt_header_pb.fields;
        let alg = jwt_header_fields.get("alg");
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
        match jwt_header_fields.get("kid") {
            Some(kid) => {
                assert!(kid.has_string_value());
                eprintln!("kid is {}", kid.get_string_value());
            }
            None => eprintln!("kid not found"),
        }
        // typ should be JWT
        match jwt_header_fields.get("typ") {
            Some(typ) => {
                assert!(typ.has_string_value());
                eprintln!("typ is {}", typ.get_string_value());
            }
            None => eprintln!("typ not found"),
        }
        assert_eq!(jwt_parts[1], JWT_PAYLOAD);
        let jwt_payload = base64::decode_config(jwt_parts[1], base64::URL_SAFE);
        assert!(jwt_payload.is_ok());
        let jwt_payload = jwt_payload.unwrap();
        let jwt_payload_s = unsafe { String::from_utf8_unchecked(jwt_payload) };
        eprintln!("JWT payload JSON:\n{}", jwt_payload_s);
        eprintln!("JWT payload expected JSON:\n{}", JWT_JSON);
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
        let _ = std::fs::File::create("./created_pb")
            .and_then(|mut f| f.write_all(bytes_out.as_slice()));
        let _ =
            std::fs::File::create("./expected_pb").and_then(|mut f| f.write_all(JWT_PAYLOAD_PB));
        let _ =
            std::fs::File::create("./metadata_pb").and_then(|mut f| f.write_all(EXAMPLE_METADATA));
        let _ = std::fs::File::create("./metadata_filter_metadata_pb")
            .and_then(|mut f| f.write_all(EXAMPLE_METADATA_FILTER_METADATA));
        let _ = std::fs::File::create("./metadata_filter_metadata_envoy_jwt_authn_pb")
            .and_then(|mut f| f.write_all(EXAMPLE_METADATA_FILTER_METADATA_ENVOY_JWT_AUTHN));
        let _ = std::fs::File::create("./metadata_filter_metadata_envoy_jwt_authn_verified_jwt_pb")
            .and_then(|mut f| {
                f.write_all(EXAMPLE_METADATA_FILTER_METADATA_ENVOY_JWT_AUTHN_VERIFIED_JWT)
            });
        let hex = bytes_out
            .chunks(16)
            .map(|line_bytes| {
                line_bytes
                    .iter()
                    .map(|c| format!("0x{:02x}", *c))
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .collect::<Vec<_>>()
            .join("\n");
        eprintln!("Payload PB bytes (len {}): [\n{}\n]", bytes_out.len(), hex);
        let hex = JWT_PAYLOAD_PB
            .chunks(16)
            .map(|line_bytes| {
                line_bytes
                    .iter()
                    .map(|c| format!("{:02x}", *c))
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .collect::<Vec<_>>()
            .join("\n");
        eprintln!(
            "Payload PB envoy (len {}): [\n{}\n]",
            JWT_PAYLOAD_PB.len(),
            hex
        );
        eprintln!("ok");
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
        let jwt_json = JWT_JSON;
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
