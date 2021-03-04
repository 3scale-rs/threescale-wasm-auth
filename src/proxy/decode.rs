//use protobuf::Message;
use prost::Message;
use std::{borrow::Cow, intrinsics::copy_nonoverlapping};
use thiserror::Error;

use crate::configuration::Decode;

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Metadata {
    /// Key is the reverse DNS filter name, e.g. com.acme.widget. The envoy.*
    /// namespace is reserved for Envoy's built-in filters.
    #[prost(map = "string, message", tag = "1")]
    pub filter_metadata: ::std::collections::HashMap<std::string::String, ::prost_types::Struct>,
}

pub struct Pairs {
    pairs: Vec<(String, String)>,
}

impl Pairs {
    pub fn new() -> Self {
        Self { pairs: vec![] }
    }

    pub fn decode(b: &mut [u8]) -> Self {
        let mut b32 = b as *const _ as *const u32;
        let pairs_len = unsafe { *b32 } as usize;
        let mut pairs = Vec::with_capacity(pairs_len);
        let mut b8 = unsafe { b32.offset(pairs_len as isize * 2 + 1) as *const u8 };
        for _ in 0..pairs_len {
            unsafe {
                b32 = b32.add(1);
                let k_len = *b32 as usize;
                b32 = b32.add(1);
                let v_len = *b32 as usize;
                let mut k = String::with_capacity(k_len);
                let mut v = String::with_capacity(v_len);
                core::ptr::copy_nonoverlapping(b8, k.as_mut_ptr(), k_len);
                b8 = b8.add(k_len + 1);
                core::ptr::copy_nonoverlapping(b8, v.as_mut_ptr(), v_len);
                b8 = b8.add(v_len + 1);
                pairs.push((k, v));
            }
        }

        Self { pairs }
    }

    pub fn encode(&self, b: &mut [u8]) -> Result<(), ()> {
        let buf_len = b.len();
        let pairs_len = self.pairs.len();
        let required_len = self.pairs.iter(pairs_len).fold(0, |acc, (k, v) {
            acc += k.len().saturating_add(v.len()).saturating_add(2)
        });
        let mut required_len =
            pairs_len * 2 * core::mem::size_of::<u32>() + core::mem::size_of::<u32>();
        if buf_len < required_len {
            return Err(());
        }
        let mut b32 = b as *mut _ as *mut u32; // XXX almost surely UB
        unsafe { *b32 = pairs_len as u32 };
        for (k, v) in &self.pairs {
            unsafe {
                b32 = b32.add(1);
                *b32 = k.len() as u32;
                b32 = b32.add(1);
                *b32 = v.len() as u32;
                required_len = required_len
                    .saturating_add(k.len())
                    .saturating_add(v.len())
                    .saturating_add(2);
            }
        }
        let mut b8 = b32 as *mut u8;
        for (k, v) in &self.pairs {
            unsafe {
                std::ptr::copy_nonoverlapping(k.as_ptr(), b8, k.len());
                b8 = b8.add(k.len());
                *b8 = 0;
                b8 = b8.add(1);
                std::ptr::copy_nonoverlapping(v.as_ptr(), b8, v.len());
                b8 = b8.add(v.len());
                *b8 = 0;
                b8 = b8.add(1);
            }
        }

        Ok(())
    }
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
}

impl<'a> Value<'a> {
    pub fn to_string(self) -> Option<String> {
        match self {
            Value::String(s) => Some(s.into_owned()),
            Value::Bytes(b) => String::from_utf8(b.into_owned()).ok(),
            Value::JsonValue(json) => json.as_str().map(|s| s.to_string()),
            Value::ProtoValue(mut proto) => {
                //if proto.has_string_value() {
                //    //proto.take_string_value().into()
                //    None
                //} else if proto.has_struct_value() {
                //let s = proto.take_struct_value();
                log::warn!("STRUCT FOUND?"); //: {:#?}", s);
                None
                //} else {
                //    None
                //}
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

        let hex = bytes
            .iter()
            .map(|c| format!("{:#02x?}", *c))
            .collect::<Vec<_>>()
            .join(", ");
        log::debug!("Decoding {} bytes: [{}]", bytes.len(), hex);
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
