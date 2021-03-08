use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Operation {
    Decode {
        input: Format,
        kind: Decode,
        output: Format,
    },
    Lookup {
        input: Format,
        key: String,
        out: Format,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Location {
    Header {
        keys: Vec<String>,
        decode: Option<Vec<Decode>>,
    },
    QueryString {
        keys: Vec<String>,
        decode: Option<Vec<Decode>>,
    },
    //Body,
    //Trailer,
    Property {
        path: Vec<String>,
        format: Format,
        lookup: Option<Vec<(String, Format)>>,
        decode: Option<Vec<Decode>>,
    },
    //Any,
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Decode {
    #[serde(rename = "base64dec")]
    Base64Decode,
    #[serde(rename = "base64urldec")]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct ValueDnF {
    pub decode: Option<Vec<Decode>>,
    pub format: Option<Format>,
}

impl ValueDnF {
    pub fn decode(&self) -> Option<&Vec<Decode>> {
        self.decode.as_ref()
    }

    pub fn format(&self) -> Option<Format> {
        self.format
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct LocationInfo {
    #[serde(flatten)]
    pub location: Location,
    pub path: Option<Vec<String>>,
    #[serde(flatten)]
    pub value_dnf: ValueDnF,
}

impl LocationInfo {
    pub fn location(&self) -> &Location {
        &self.location
    }

    pub fn path(&self) -> Option<&Vec<String>> {
        self.path.as_ref()
    }
    pub fn value_dnf(&self) -> &ValueDnF {
        &self.value_dnf
    }
}
