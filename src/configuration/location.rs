use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Operation {
    Decode(Decode),
    Lookup {
        input: Format,
        key: String,
        output: Format,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Location {
    Header {
        keys: Vec<String>,
        ops: Option<Vec<Operation>>,
    },
    QueryString {
        keys: Vec<String>,
        ops: Option<Vec<Operation>>,
    },
    Property {
        path: Vec<String>,
        format: Format,
        keys: Vec<String>,
        ops: Option<Vec<Operation>>,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Decode {
    #[serde(rename = "base64")]
    Base64Decode,
    #[serde(rename = "base64_urlsafe")]
    Base64URLDecode,
    #[serde(rename = "protobuf")]
    ProtobufValue,
    #[serde(rename = "json")]
    JsonValue,
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Format {
    String,
    Base64String,
    Json,
    ProtobufStruct,
    Pairs,
}
