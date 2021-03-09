//use protobuf::Message;
use prost::Message;
use std::borrow::Cow;
use thiserror::Error;

use crate::configuration::{Decode, Operation};
use crate::util::pairs::Pairs;

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Metadata {
    /// Key is the reverse DNS filter name, e.g. com.acme.widget. The envoy.*
    /// namespace is reserved for Envoy's built-in filters.
    #[prost(map = "string, message", tag = "1")]
    pub filter_metadata: ::std::collections::HashMap<std::string::String, ::prost_types::Struct>,
}

#[derive(Debug, Error)]
pub(crate) enum ValueError<'a> {
    #[error("can only decode strings or bytes")]
    Type(Value<'a>),
    #[error("error decoding base64")]
    DecodeBase64(Value<'a>, #[source] base64::DecodeError),
    #[error("error decoding protobuf")]
    //DecodeProtobuf(Value<'a>, #[source] protobuf::ProtobufError),
    DecodeProtobuf(Value<'a>, #[source] prost::DecodeError),
    #[error("error decoding JSON")]
    DecodeJSON(Value<'a>, #[source] serde_json::Error),
    #[error("error decoding pairs")]
    DecodePairs(Value<'a>),
}

#[derive(Debug, Clone)]
pub(crate) enum Value<'a> {
    Bytes(Cow<'a, [u8]>),
    String(Cow<'a, str>),
    //ProtoValue(protobuf::well_known_types::Struct),
    //ProtoValue(prost_types::Struct),
    ProtoValue(Metadata),
    //ProtoList(protobuf::well_known_types::ListValue),
    //ProtoStruct(HashMap<String, protobuf::well_known_types::Value>),
    //ProtoString(protobuf::well_known_types::StringValue),
    JsonValue(serde_json::Value),
    //JsonString(serde_json::Value::String),
    //JsonList(serde_json::Value::Array(Vec<serde_json::Value>)),
    //JsonObject(serde_json::Value::Object(serde_json::Map<String, serde_json::Value>)),
    PairsValue(Pairs),
}

impl<'a> Value<'a> {
    pub fn to_string(self) -> Option<String> {
        match self {
            Value::String(s) => Some(s.into_owned()),
            Value::Bytes(b) => String::from_utf8(b.into_owned()).ok(),
            Value::PairsValue(_p) => {
                log::error!("need to implement Pairs -> String conversion");
                unimplemented!("need to implement Pairs -> String conversion");
            }
            Value::JsonValue(json) => json.as_str().map(|s| s.to_string()),
            Value::ProtoValue(mut _proto) => {
                //if proto.has_string_value() {
                //    //proto.take_string_value().into()
                //    None
                //} else if proto.has_struct_value() {
                //let s = proto.take_struct_value();
                //log::warn!("STRUCT FOUND?"); //: {:#?}", s);
                //} else {
                //    None
                //}
                log::error!("need to implement Protobuf -> String conversion");
                unimplemented!("need to implement Protobuf -> String conversion");
            }
        }
    }

    fn decode(self, decode: Decode) -> Result<Value<'a>, ValueError<'a>> {
        // convert to bytes - decode always operates on string or bytes values, so this should work in a well formed pipeline of ops
        let bytes = match self.as_bytes() {
            Some(bytes) => bytes,
            None => return Err(ValueError::Type(self)),
        };

        let hex = bytes
            .iter()
            .map(|c| format!("0x{:02x}", *c))
            .collect::<Vec<_>>()
            .join(", ");
        log::debug!("Decoding {} bytes: [{}]", bytes.len(), hex);

        let res = match decode {
            Decode::Base64Decode => Value::Bytes(Cow::from(
                base64::decode_config(bytes, base64::STANDARD)
                    .map_err(|e| ValueError::DecodeBase64(self, e))?,
            )),
            Decode::Base64URLDecode => Value::Bytes(Cow::from(
                base64::decode_config(bytes, base64::URL_SAFE)
                    .map_err(|e| ValueError::DecodeBase64(self, e))?,
            )),
            Decode::ProtobufValue => {
                //let proto = {
                //    let mut cis = protobuf::CodedInputStream::from_bytes(bytes);
                //    cis.read_message::<protobuf::well_known_types::Struct>()
                //};
                //let proto = <prost_types::Struct as prost::Message>::decode(bytes);
                let proto = Metadata::decode(bytes);

                log::warn!("protobuf parsing result: {:#?}", proto);
                match proto {
                    Ok(value) => {
                        //let type_id = value.type_id();
                        //log::warn!("protobuf type id {:#?}", type_id);
                        //if value.has_struct_value() {
                        //    log::warn!("protobuf has struct")
                        //} else {
                        //    log::warn!("protobuf has struct FAILED")
                        //}
                        log::warn!("===> parsed ok!!!");
                        Value::ProtoValue(value)
                    }
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

        Ok(res)
    }
    pub fn perform_op(self, op: Option<Operation>) -> Result<Value<'a>, ValueError<'a>> {
        let op = match op {
            Some(op) => op,
            None => return Ok(self),
        };
        let value = match op {
            Operation::Decode(d) => self.decode(d),
            Operation::Lookup { input, key, output } => unimplemented!("ei hoh"),
        };

        value
    }

    pub fn decode_multiple(
        self,
        ops: Option<&Vec<Operation>>,
    ) -> Result<Value<'a>, ValueError<'a>> {
        match ops {
            Some(decode_vec) => decode_vec
                .iter()
                .try_fold(self, |val, &op| val.perform_op(Some(op))),
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
