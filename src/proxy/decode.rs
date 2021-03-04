//use protobuf::Message;
use prost::Message;
use std::borrow::Cow;
use thiserror::Error;

use crate::configuration::Decode;

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Metadata {
    /// Key is the reverse DNS filter name, e.g. com.acme.widget. The envoy.*
    /// namespace is reserved for Envoy's built-in filters.
    #[prost(map = "string, message", tag = "1")]
    pub filter_metadata: ::std::collections::HashMap<std::string::String, ::prost_types::Struct>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Pairs {
    pairs: Vec<(String, String)>,
}

impl Pairs {
    pub fn new(pairs: Vec<(String, String)>) -> Self {
        Self { pairs }
    }

    // None retval means usize can't represent the length requirement
    fn required_buffer_length(&self) -> Option<usize> {
        self.pairs
            .iter()
            .try_fold(core::mem::size_of::<u32>(), |acc, (k, v)| {
                acc.checked_add(
                    k.len()
                        .saturating_add(v.len())
                        .saturating_add(2)
                        .saturating_mul(core::mem::size_of::<u32>()),
                )
            })
    }

    // Returns Self or error with a hint to the very minimum required buffer length.
    // Note: minimum required length can vary as data is parsed, so buffers should at least ensure
    //       that many bytes are available before calling again (for which there could be another
    //       bigger requirement).
    pub fn decode(b: &mut [u8]) -> Result<Self, usize> {
        let buf_len = b.len();
        // ensure min length of 1 u32
        if buf_len < core::mem::size_of::<u32>() {
            return Err(core::mem::size_of::<u32>());
        }
        let mut b32 = b as *const _ as *const u32;
        // read number of pairs
        let pairs_len = unsafe { *b32 } as usize;
        // minimum required length is now 1 + pairs_len * 2 (for k and v lens) * sizeof(u32) + pairs_len * 2 (for k and v zero-termination) * sizeof(u8)
        let required_len = core::mem::size_of::<u32>()
            + pairs_len * 2 * core::mem::size_of::<u32>()
            + pairs_len * 2 * core::mem::size_of::<u8>();
        if buf_len < required_len {
            return Err(required_len);
        }
        let mut pairs = Vec::with_capacity(pairs_len);
        let required_len = (0..pairs_len)
            .try_fold(required_len, |acc, _| {
                let (k_len, v_len) = unsafe {
                    b32 = b32.add(1);
                    let k_len = *b32 as usize;
                    b32 = b32.add(1);
                    let v_len = *b32 as usize;
                    pairs.push((String::with_capacity(k_len), String::with_capacity(v_len)));
                    (k_len, v_len)
                };
                acc.checked_add(
                    k_len
                        .saturating_add(v_len)
                        .saturating_add(2 * core::mem::size_of::<u8>()),
                )
            })
            .ok_or(usize::MAX)?;
        if buf_len < required_len {
            return Err(required_len - buf_len);
        }
        let mut b8 = unsafe { b32.offset(1) } as *const u8;
        for (k, v) in &mut pairs {
            unsafe {
                core::ptr::copy_nonoverlapping(b8, k.as_mut_ptr(), k.len());
                b8 = b8.add(k.len() + 1);
                core::ptr::copy_nonoverlapping(b8, v.as_mut_ptr(), v.len());
                b8 = b8.add(v.len() + 1);
            }
        }

        Ok(Self { pairs })
    }

    // Encodes the pairs into a buffer, returns error with amount of bytes short of requirements.
    // Err(usize::MAX) is there is no way this can work
    pub fn encode(&self, b: &mut [u8]) -> Result<(), usize> {
        let buf_len = b.len();
        let pairs_len = self.pairs.len();
        let required_len = self.required_buffer_length().ok_or(usize::MAX)?;
        if buf_len < required_len {
            return Err(required_len - buf_len);
        }
        let mut b32 = b as *mut _ as *mut u32;
        // write number of pairs
        unsafe { *b32 = pairs_len as u32 };
        // write all keylen, valuelen
        for (k, v) in &self.pairs {
            unsafe {
                b32 = b32.add(1);
                *b32 = k.len() as u32;
                b32 = b32.add(1);
                *b32 = v.len() as u32;
            }
        }
        let mut b8 = b32 as *mut u8;
        for (k, v) in &self.pairs {
            unsafe {
                std::ptr::copy_nonoverlapping(k.as_ptr(), b8, k.len());
                // zero-terminate
                b8 = b8.add(k.len());
                *b8 = 0;
                b8 = b8.add(1);
                std::ptr::copy_nonoverlapping(v.as_ptr(), b8, v.len());
                // zero-terminate
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
            Value::PairsValue(p) => {
                let s = String::with_capacity(1024)
                let r = p.encode(s.as_bytes_mut()).unwrap();
                s
            }
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
