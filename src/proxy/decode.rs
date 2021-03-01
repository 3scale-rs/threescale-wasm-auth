use std::borrow::Cow;
use thiserror::Error;

use crate::configuration::Decode;

#[derive(Debug, Error)]
pub(crate) enum ValueError<'a> {
    #[error("can only decode strings or bytes")]
    Type(Value<'a>),
    #[error("error decoding base64")]
    DecodeBase64(Value<'a>, #[source] base64::DecodeError),
    #[error("error decoding protobuf")]
    DecodeProtobuf(Value<'a>, #[source] protobuf::ProtobufError),
    #[error("error decoding JSON")]
    DecodeJSON(Value<'a>, #[source] serde_json::Error),
}

#[derive(Debug, Clone)]
pub(crate) enum Value<'a> {
    Bytes(Cow<'a, [u8]>),
    String(Cow<'a, str>),
    ProtoValue(protobuf::well_known_types::Value),
    //ProtoList(protobuf::well_known_types::ListValue),
    //ProtoStruct(HashMap<String, protobuf::well_known_types::Value>),
    //ProtoString(protobuf::well_known_types::StringValue),
    JsonValue(serde_json::Value),
    //JsonString(serde_json::Value::String),
    //JsonList(serde_json::Value::Array(Vec<serde_json::Value>)),
    //JsonObject(serde_json::Value::Object(serde_json::Map<String, serde_json::Value>)),
}

impl<'a> Value<'a> {
    pub fn to_string(self) -> Option<String> {
        match self {
            Value::String(s) => Some(s.into_owned()),
            Value::Bytes(b) => String::from_utf8(b.into_owned()).ok(),
            Value::JsonValue(json) => json.as_str().map(|s| s.to_string()),
            Value::ProtoValue(mut proto) => {
                if proto.has_string_value() {
                    proto.take_string_value().into()
                } else {
                    None
                }
            }
        }
    }

    pub fn decode(self, decode: Option<Decode>) -> Result<Value<'a>, ValueError<'a>> {
        if let None = decode {
            return Ok(self);
        }
        let bytes = match self.as_bytes() {
            Some(bytes) => bytes,
            None => return Err(ValueError::Type(self)),
        };

        let decode = decode.unwrap();
        let value = match decode {
            Decode::Base64Decode => Value::Bytes(Cow::from(
                base64::decode_config(bytes, base64::STANDARD)
                    .map_err(|e| ValueError::DecodeBase64(self, e))?,
            )),
            Decode::Base64URLDecode => Value::Bytes(Cow::from(
                base64::decode_config(bytes, base64::URL_SAFE)
                    .map_err(|e| ValueError::DecodeBase64(self, e))?,
            )),
            Decode::ProtobufValue => {
                let proto = {
                    let mut cis = protobuf::CodedInputStream::from_bytes(bytes);
                    cis.read_message::<protobuf::well_known_types::Value>()
                };

                match proto {
                    Ok(value) => Value::ProtoValue(value),
                    Err(e) => Err(ValueError::DecodeProtobuf(self, e))?,
                }
            }
            Decode::JsonValue => {
                let json = serde_json::from_slice::<serde_json::Value>(bytes);
                match json {
                    Ok(value) => Value::JsonValue(value),
                    Err(e) => Err(ValueError::DecodeJSON(self, e))?,
                }
            }
        };

        Ok(value)
    }

    pub fn decode_multiple(
        self,
        decode_vec: Option<&Vec<Decode>>,
    ) -> Result<Value<'a>, ValueError<'a>> {
        match decode_vec {
            Some(decode_vec) => decode_vec
                .iter()
                .try_fold(self, |val, &decode| val.decode(Some(decode))),
            _ => Ok(self),
        }
    }

    fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Bytes(v) => v.as_ref().into(),
            Value::String(v) => v.as_bytes().into(),
            _ => None,
        }
    }
}
