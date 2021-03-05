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
    pub fn decode(b: &[u8]) -> Result<Self, usize> {
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
            .checked_add(
                pairs_len
                    .saturating_mul(2)
                    .saturating_mul(core::mem::size_of::<u32>())
                    .saturating_add(
                        pairs_len
                            .saturating_mul(2)
                            .saturating_mul(core::mem::size_of::<u8>()),
                    ),
            )
            .ok_or(usize::MAX)?;
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
                acc.checked_add(k_len.saturating_add(v_len))
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

    // Encodes the pairs into a buffer, returns error with amount of bytes short of requirements, Ok(written_bytes) otherwise
    // Err(usize::MAX) is there is no way this can work
    pub fn encode(&self, b: &mut [u8]) -> Result<usize, usize> {
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

        Ok(required_len)
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
                let mut s = String::with_capacity(1024);
                // XXX FIXME Note: the buffer might be written to, but it won't make a proper string because len has not been... updated.
                let r = p.encode(unsafe { s.as_bytes_mut() }).unwrap();
                Some(s)
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
            .map(|c| format!("0x{:02x}", *c))
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

#[cfg(test)]
mod test {
    use super::*;

    mod fixtures {
        // this is a Protobuf::Struct
        pub const EXAMPLE_METADATA: &[u8] = &[
            0x0a, 0xcf, 0x03, 0x0a, 0x1c, 0x65, 0x6e, 0x76, 0x6f, 0x79, 0x2e, 0x66, 0x69, 0x6c,
            0x74, 0x65, 0x72, 0x73, 0x2e, 0x68, 0x74, 0x74, 0x70, 0x2e, 0x6a, 0x77, 0x74, 0x5f,
            0x61, 0x75, 0x74, 0x68, 0x6e, 0x12, 0xae, 0x03, 0x0a, 0xab, 0x03, 0x0a, 0x0c, 0x76,
            0x65, 0x72, 0x69, 0x66, 0x69, 0x65, 0x64, 0x5f, 0x6a, 0x77, 0x74, 0x12, 0x9a, 0x03,
            0x2a, 0x97, 0x03, 0x0a, 0x31, 0x0a, 0x03, 0x69, 0x73, 0x73, 0x12, 0x2a, 0x1a, 0x28,
            0x68, 0x74, 0x74, 0x70, 0x73, 0x3a, 0x2f, 0x2f, 0x6b, 0x65, 0x79, 0x63, 0x6c, 0x6f,
            0x61, 0x6b, 0x3a, 0x38, 0x34, 0x34, 0x33, 0x2f, 0x61, 0x75, 0x74, 0x68, 0x2f, 0x72,
            0x65, 0x61, 0x6c, 0x6d, 0x73, 0x2f, 0x6d, 0x61, 0x73, 0x74, 0x65, 0x72, 0x0a, 0x10,
            0x0a, 0x03, 0x69, 0x61, 0x74, 0x12, 0x09, 0x11, 0x00, 0x00, 0x00, 0xf1, 0x96, 0x10,
            0xd8, 0x41, 0x0a, 0x1d, 0x0a, 0x12, 0x70, 0x72, 0x65, 0x66, 0x65, 0x72, 0x72, 0x65,
            0x64, 0x5f, 0x75, 0x73, 0x65, 0x72, 0x6e, 0x61, 0x6d, 0x65, 0x12, 0x07, 0x1a, 0x05,
            0x61, 0x64, 0x6d, 0x69, 0x6e, 0x0a, 0x0a, 0x0a, 0x03, 0x61, 0x63, 0x72, 0x12, 0x03,
            0x1a, 0x01, 0x31, 0x0a, 0x0b, 0x0a, 0x03, 0x74, 0x79, 0x70, 0x12, 0x04, 0x1a, 0x02,
            0x49, 0x44, 0x0a, 0x23, 0x0a, 0x07, 0x61, 0x74, 0x5f, 0x68, 0x61, 0x73, 0x68, 0x12,
            0x18, 0x1a, 0x16, 0x33, 0x66, 0x4c, 0x79, 0x36, 0x32, 0x67, 0x35, 0x76, 0x57, 0x42,
            0x37, 0x53, 0x4a, 0x4e, 0x4b, 0x62, 0x4c, 0x44, 0x2d, 0x55, 0x41, 0x0a, 0x10, 0x0a,
            0x03, 0x65, 0x78, 0x70, 0x12, 0x09, 0x11, 0x00, 0x00, 0x00, 0x00, 0x97, 0x10, 0xd8,
            0x41, 0x0a, 0x0d, 0x0a, 0x03, 0x61, 0x75, 0x64, 0x12, 0x06, 0x1a, 0x04, 0x74, 0x65,
            0x73, 0x74, 0x0a, 0x2d, 0x0a, 0x03, 0x73, 0x75, 0x62, 0x12, 0x26, 0x1a, 0x24, 0x35,
            0x34, 0x33, 0x32, 0x31, 0x33, 0x63, 0x36, 0x2d, 0x31, 0x62, 0x30, 0x34, 0x2d, 0x34,
            0x33, 0x63, 0x31, 0x2d, 0x38, 0x38, 0x61, 0x61, 0x2d, 0x32, 0x62, 0x66, 0x65, 0x31,
            0x39, 0x64, 0x62, 0x38, 0x65, 0x31, 0x62, 0x0a, 0x2d, 0x0a, 0x03, 0x6a, 0x74, 0x69,
            0x12, 0x26, 0x1a, 0x24, 0x66, 0x34, 0x66, 0x38, 0x33, 0x64, 0x32, 0x63, 0x2d, 0x30,
            0x61, 0x63, 0x31, 0x2d, 0x34, 0x34, 0x30, 0x37, 0x2d, 0x61, 0x35, 0x66, 0x36, 0x2d,
            0x37, 0x30, 0x34, 0x38, 0x39, 0x37, 0x34, 0x30, 0x39, 0x32, 0x62, 0x62, 0x0a, 0x16,
            0x0a, 0x09, 0x61, 0x75, 0x74, 0x68, 0x5f, 0x74, 0x69, 0x6d, 0x65, 0x12, 0x09, 0x11,
            0x00, 0x00, 0xc0, 0xf0, 0x96, 0x10, 0xd8, 0x41, 0x0a, 0x14, 0x0a, 0x0e, 0x65, 0x6d,
            0x61, 0x69, 0x6c, 0x5f, 0x76, 0x65, 0x72, 0x69, 0x66, 0x69, 0x65, 0x64, 0x12, 0x02,
            0x20, 0x00, 0x0a, 0x0d, 0x0a, 0x03, 0x61, 0x7a, 0x70, 0x12, 0x06, 0x1a, 0x04, 0x74,
            0x65, 0x73, 0x74, 0x0a, 0x37, 0x0a, 0x0d, 0x73, 0x65, 0x73, 0x73, 0x69, 0x6f, 0x6e,
            0x5f, 0x73, 0x74, 0x61, 0x74, 0x65, 0x12, 0x26, 0x1a, 0x24, 0x66, 0x36, 0x65, 0x33,
            0x37, 0x62, 0x39, 0x63, 0x2d, 0x63, 0x65, 0x31, 0x37, 0x2d, 0x34, 0x63, 0x34, 0x38,
            0x2d, 0x61, 0x64, 0x39, 0x34, 0x2d, 0x65, 0x65, 0x33, 0x63, 0x38, 0x61, 0x38, 0x30,
            0x61, 0x39, 0x38, 0x33,
        ];
        // This should be some sort of Pairs
        #[rustfmt::skip]
        pub const EXAMPLE_METADATA_FILTER_METADATA: &[u8] = &[
            0x01, 0x00, 0x00, 0x00, // 1 pair
            0x1c, 0x00, 0x00, 0x00, // key len: 28
            0xd5, 0x01, 0x00, 0x00, // val len: 469
            0x65, 0x6e, 0x76, 0x6f, 0x79, 0x2e, 0x66, 0x69, 0x6c, 0x74, 0x65, 0x72, 0x73, 0x2e, // envoy.filters.
            0x68, 0x74, 0x74, 0x70, 0x2e, 0x6a, 0x77, 0x74, 0x5f, 0x61, 0x75, 0x74, 0x68, 0x6e, // http.jwt_authn
            0x00, // end-of-key
            0x01, 0x00, 0x00, 0x00, // 1 pair
            0x0c, 0x00, 0x00, 0x00, // key len: 12
            0xbb, 0x01, 0x00, 0x00, // val len: 443
            0x76, 0x65, 0x72, 0x69, 0x66, 0x69, 0x65, 0x64, 0x5f, 0x6a, 0x77, 0x74, 0x00, // verified_jwt
            0x0e, 0x00, 0x00, 0x00, // 14 pairs
            0x07, 0x00, 0x00, 0x00, // key len 1: 7
            0x16, 0x00, 0x00, 0x00, // val len 1: 22
            0x03, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, // kv 2
            0x03, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, // kv 3
            0x03, 0x00, 0x00, 0x00, 0x24, 0x00, 0x00, 0x00, // ...
            0x03, 0x00, 0x00, 0x00, 0x24, 0x00, 0x00, 0x00,
            0x09, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00,
            0x0e, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
            0x03, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
            0x0d, 0x00, 0x00, 0x00, 0x24, 0x00, 0x00, 0x00,
            0x03, 0x00, 0x00, 0x00, 0x28, 0x00, 0x00, 0x00,
            0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
            0x12, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00,
            0x03, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00,
            0x03, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, // kv 14
            // data
            0x61, 0x74, 0x5f, 0x68, 0x61, 0x73, 0x68, 0x00, // kv 1, key (7)
            0x33, 0x66, 0x4c, 0x79, 0x36, 0x32, 0x67, 0x35, 0x76, 0x57, // kv 1, val (22)
            0x42, 0x37, 0x53, 0x4a, 0x4e, 0x4b, 0x62, 0x4c, 0x44, 0x2d,
            0x55, 0x41, 0x00,
            0x65, 0x78, 0x70, 0x00, 0x00, 0x00, 0x00, 0x00, 0x97, 0x10,
            0xd8, 0x41, 0x00, 0x61, 0x75, 0x64, 0x00, 0x74, 0x65, 0x73,
            0x74, 0x00, 0x73, 0x75, 0x62, 0x00, 0x35, 0x34, 0x33, 0x32,
            0x31, 0x33, 0x63, 0x36, 0x2d, 0x31, 0x62, 0x30, 0x34, 0x2d,
            0x34, 0x33, 0x63, 0x31, 0x2d, 0x38, 0x38, 0x61, 0x61, 0x2d,
            0x32, 0x62, 0x66, 0x65, 0x31, 0x39, 0x64, 0x62, 0x38, 0x65,
            0x31, 0x62, 0x00, 0x6a, 0x74, 0x69, 0x00, 0x66, 0x34, 0x66,
            0x38, 0x33, 0x64, 0x32, 0x63, 0x2d, 0x30, 0x61, 0x63, 0x31,
            0x2d, 0x34, 0x34, 0x30, 0x37, 0x2d, 0x61, 0x35, 0x66, 0x36,
            0x2d, 0x37, 0x30, 0x34, 0x38, 0x39, 0x37, 0x34, 0x30, 0x39,
            0x32, 0x62, 0x62, 0x00, 0x61, 0x75, 0x74, 0x68, 0x5f, 0x74,
            0x69, 0x6d, 0x65, 0x00, 0x00, 0x00, 0xc0, 0xf0, 0x96, 0x10,
            0xd8, 0x41, 0x00, 0x65, 0x6d, 0x61, 0x69, 0x6c, 0x5f, 0x76,
            0x65, 0x72, 0x69, 0x66, 0x69, 0x65, 0x64, 0x00, 0x00, 0x00,
            0x61, 0x7a, 0x70, 0x00, 0x74, 0x65, 0x73, 0x74, 0x00, 0x73,
            0x65, 0x73, 0x73, 0x69, 0x6f, 0x6e, 0x5f, 0x73, 0x74, 0x61,
            0x74, 0x65, 0x00, 0x66, 0x36, 0x65, 0x33, 0x37, 0x62, 0x39,
            0x63, 0x2d, 0x63, 0x65, 0x31, 0x37, 0x2d, 0x34, 0x63, 0x34,
            0x38, 0x2d, 0x61, 0x64, 0x39, 0x34, 0x2d, 0x65, 0x65, 0x33,
            0x63, 0x38, 0x61, 0x38, 0x30, 0x61, 0x39, 0x38, 0x33, 0x00,
            0x69, 0x73, 0x73, 0x00, 0x68, 0x74, 0x74, 0x70, 0x73, 0x3a,
            0x2f, 0x2f, 0x6b, 0x65, 0x79, 0x63, 0x6c, 0x6f, 0x61, 0x6b,
            0x3a, 0x38, 0x34, 0x34, 0x33, 0x2f, 0x61, 0x75, 0x74, 0x68,
            0x2f, 0x72, 0x65, 0x61, 0x6c, 0x6d, 0x73, 0x2f, 0x6d, 0x61,
            0x73, 0x74, 0x65, 0x72, 0x00, 0x61, 0x63, 0x72, 0x00, 0x31,
            0x00, 0x70, 0x72, 0x65, 0x66, 0x65, 0x72, 0x72, 0x65, 0x64,
            0x5f, 0x75, 0x73, 0x65, 0x72, 0x6e, 0x61, 0x6d, 0x65, 0x00,
            0x61, 0x64, 0x6d, 0x69, 0x6e, 0x00, 0x69, 0x61, 0x74, 0x00,
            0x00, 0x00, 0x00, 0xf1, 0x96, 0x10, 0xd8, 0x41, 0x00, 0x74,
            0x79, 0x70, 0x00, 0x49, 0x44, 0x00, 0x00, 0x00,
        ];
        // val:  8 len: 1614961603
        //0x00, 0x00, 0xc0, 0xf0, 0x96, 0x10, 0xd8, 0x41, 0x00,
        // 0000 0000, 0000 0000, 1100 0000, 1111 0000, 1001 0110, 0001 0000, 1101 1000, 0100 0001
        //    6             5            4              3            2             1            0
        // seee eeee  eeee ffff  ffff ffff  ffff ffff  ffff ffff  ffff ffff  ffff ffff  ffff ffff
        //
        // 0100 0001, 1101 1000, 0001 0000, 1001 0110, 1111 0000, 1100 0000, 0000 0000, 0000 0000
        //
        // s = 0
        // e = 100 0001 1101 = 1053 - 1023 = 30
        // p = 1000 0001 0000 1001 0110 1111 0000 1100 0000 0000 0000 0000 0000
        // p = 1000000100001001011011110000110000000000000000000000 = 2270040283938816
        // 2270040283938816 x 2^e
        //
        // val:  8 len: 1614961604
        //0x00, 0x00, 0x00, 0xf1, 0x96, 0x10, 0xd8, 0x41, 0x00,
        //            0000 0000, 1111 0001, 1001 0110 +1  (0000 0001)
        //
        // 1614961603: 1100 0011 0101 1011 0100 0010 0110 0000 (LSB: 0xc35b4260)
        // 1614961604: 1100 0100 0101 1011 0100 0010 0110 0000 (LSB: 0xc45b4260)
        //
        // val:  8 len: 1614961664
        //0x00, 0x00, 0x00, 0x00, 0x97, 0x10, 0xd8, 0x41, 0x00,
        //            0000 0000, 0000 0000, 1001 0111 +60 (0011 1100)
        //
        // 0x97 (151) 1001 011(1) 0000 0000 0000 0000
        // - 3c ( 60)
        //        91

        // This should be some sort of Pairs
        pub const EXAMPLE_METADATA_FILTER_ENVOY_JWT_AUTHN: &[u8] = &[
            0x01, 0x00, 0x00, 0x00, // 1 pair
            0x0c, 0x00, 0x00, 0x00, // key len is 12
            0xbb, 0x01, 0x00, 0x00, // value len is 443
            // pair[0] key: len 12: verified_jwt
            0x76, 0x65, 0x72, 0x69, 0x66, 0x69, 0x65, 0x64, 0x5f, 0x6a, 0x77, 0x74, 0x00,
            // verified_jwt entry -> should equal the verified_jwt value below
            0x0e, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x16, 0x00, 0x00, 0x00, 0x03, 0x00,
            0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
            0x03, 0x00, 0x00, 0x00, 0x24, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x24, 0x00,
            0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x0e, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x0d, 0x00,
            0x00, 0x00, 0x24, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x28, 0x00, 0x00, 0x00,
            0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x12, 0x00, 0x00, 0x00, 0x05, 0x00,
            0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00,
            0x02, 0x00, 0x00, 0x00, 0x61, 0x74, 0x5f, 0x68, 0x61, 0x73, 0x68, 0x00, 0x33, 0x66,
            0x4c, 0x79, 0x36, 0x32, 0x67, 0x35, 0x76, 0x57, 0x42, 0x37, 0x53, 0x4a, 0x4e, 0x4b,
            0x62, 0x4c, 0x44, 0x2d, 0x55, 0x41, 0x00, 0x65, 0x78, 0x70, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x97, 0x10, 0xd8, 0x41, 0x00, 0x61, 0x75, 0x64, 0x00, 0x74, 0x65, 0x73, 0x74,
            0x00, 0x73, 0x75, 0x62, 0x00, 0x35, 0x34, 0x33, 0x32, 0x31, 0x33, 0x63, 0x36, 0x2d,
            0x31, 0x62, 0x30, 0x34, 0x2d, 0x34, 0x33, 0x63, 0x31, 0x2d, 0x38, 0x38, 0x61, 0x61,
            0x2d, 0x32, 0x62, 0x66, 0x65, 0x31, 0x39, 0x64, 0x62, 0x38, 0x65, 0x31, 0x62, 0x00,
            0x6a, 0x74, 0x69, 0x00, 0x66, 0x34, 0x66, 0x38, 0x33, 0x64, 0x32, 0x63, 0x2d, 0x30,
            0x61, 0x63, 0x31, 0x2d, 0x34, 0x34, 0x30, 0x37, 0x2d, 0x61, 0x35, 0x66, 0x36, 0x2d,
            0x37, 0x30, 0x34, 0x38, 0x39, 0x37, 0x34, 0x30, 0x39, 0x32, 0x62, 0x62, 0x00, 0x61,
            0x75, 0x74, 0x68, 0x5f, 0x74, 0x69, 0x6d, 0x65, 0x00, 0x00, 0x00, 0xc0, 0xf0, 0x96,
            0x10, 0xd8, 0x41, 0x00, 0x65, 0x6d, 0x61, 0x69, 0x6c, 0x5f, 0x76, 0x65, 0x72, 0x69,
            0x66, 0x69, 0x65, 0x64, 0x00, 0x00, 0x00, 0x61, 0x7a, 0x70, 0x00, 0x74, 0x65, 0x73,
            0x74, 0x00, 0x73, 0x65, 0x73, 0x73, 0x69, 0x6f, 0x6e, 0x5f, 0x73, 0x74, 0x61, 0x74,
            0x65, 0x00, 0x66, 0x36, 0x65, 0x33, 0x37, 0x62, 0x39, 0x63, 0x2d, 0x63, 0x65, 0x31,
            0x37, 0x2d, 0x34, 0x63, 0x34, 0x38, 0x2d, 0x61, 0x64, 0x39, 0x34, 0x2d, 0x65, 0x65,
            0x33, 0x63, 0x38, 0x61, 0x38, 0x30, 0x61, 0x39, 0x38, 0x33, 0x00, 0x69, 0x73, 0x73,
            0x00, 0x68, 0x74, 0x74, 0x70, 0x73, 0x3a, 0x2f, 0x2f, 0x6b, 0x65, 0x79, 0x63, 0x6c,
            0x6f, 0x61, 0x6b, 0x3a, 0x38, 0x34, 0x34, 0x33, 0x2f, 0x61, 0x75, 0x74, 0x68, 0x2f,
            0x72, 0x65, 0x61, 0x6c, 0x6d, 0x73, 0x2f, 0x6d, 0x61, 0x73, 0x74, 0x65, 0x72, 0x00,
            0x61, 0x63, 0x72, 0x00, 0x31, 0x00, 0x70, 0x72, 0x65, 0x66, 0x65, 0x72, 0x72, 0x65,
            0x64, 0x5f, 0x75, 0x73, 0x65, 0x72, 0x6e, 0x61, 0x6d, 0x65, 0x00, 0x61, 0x64, 0x6d,
            0x69, 0x6e, 0x00, 0x69, 0x61, 0x74, 0x00, 0x00, 0x00, 0x00, 0xf1, 0x96, 0x10, 0xd8,
            0x41, 0x00, 0x74, 0x79, 0x70, 0x00, 0x49, 0x44, 0x00, 0x00,
        ];
        // This is Pairs with string and int values
        #[rustfmt::skip]
        pub const EXAMPLE_VERIFIED_JWT: &[u8] = &[
            0x0e, 0x00, 0x00, 0x00, // 14 pairs
            // pair[0] lengths
            0x07, 0x00, 0x00, 0x00, // key len 7
            0x16, 0x00, 0x00, 0x00, // val len 22
            // pair[1] lengths
            0x03, 0x00, 0x00, 0x00, // key len 3
            0x08, 0x00, 0x00, 0x00, // val len 8
            // pair[2] lengths
            0x03, 0x00, 0x00, 0x00, // key len 3
            0x04, 0x00, 0x00, 0x00, // val len 4
            // pair[3] lengths
            0x03, 0x00, 0x00, 0x00, // key len 3
            0x24, 0x00, 0x00, 0x00, // val len 36
            // pair[4] lengths
            0x03, 0x00, 0x00, 0x00, // key len 3
            0x24, 0x00, 0x00, 0x00, // val len 36
            // pair[5] lengths
            0x09, 0x00, 0x00, 0x00, // key len 9
            0x08, 0x00, 0x00, 0x00, // val len 8
            // pair[6] lengths
            0x0e, 0x00, 0x00, 0x00, // key len 14
            0x01, 0x00, 0x00, 0x00, // val len 1
            // pair[7] lengths
            0x03, 0x00, 0x00, 0x00, // key len 3
            0x04, 0x00, 0x00, 0x00, // val len 4
            // pair[8] lengths
            0x0d, 0x00, 0x00, 0x00, // key len 13
            0x24, 0x00, 0x00, 0x00, // val len 36
            // pair[9] lengths
            0x03, 0x00, 0x00, 0x00, // key len 3
            0x28, 0x00, 0x00, 0x00, // val len 40
            // pair[10] lengths
            0x03, 0x00, 0x00, 0x00, // key len 3
            0x01, 0x00, 0x00, 0x00, // val len 1
            // pair[11] lengths
            0x12, 0x00, 0x00, 0x00, // key len 18
            0x05, 0x00, 0x00, 0x00, // val len 5
            // pair[12] lengths
            0x03, 0x00, 0x00, 0x00, // key len 3
            0x08, 0x00, 0x00, 0x00, // val len 8
            // pair[13] lengths
            0x03, 0x00, 0x00, 0x00, // key len 3
            0x02, 0x00, 0x00, 0x00, // val len 2
            // pair[0] key:  7 len, at_hash
            0x61, 0x74, 0x5f, 0x68, 0x61, 0x73, 0x68, 0x00,
            // pair[0] val: 22 len, 3fLy62g5vWB7SJNKbLD-UA
            0x33, 0x66, 0x4c, 0x79, 0x36, 0x32, 0x67, 0x35, 0x76, 0x57,
            0x42, 0x37, 0x53, 0x4a, 0x4e, 0x4b, 0x62, 0x4c, 0x44, 0x2d,
            0x55, 0x41, 0x00,
            // pair[1] key:  3 len: exp
            0x65, 0x78, 0x70, 0x00,
            // pair[1] val:  8 len: f64: 1614961664
            0x00, 0x00, 0x00, 0x00, 0x97, 0x10, 0xd8, 0x41, 0x00,
            // pair[2] key:  3 len: aud
            0x61, 0x75, 0x64, 0x00,
            // pair[2] val:  4 len: test
            0x74, 0x65, 0x73, 0x74, 0x00,
            // pair[3] key:  3 len: sub
            0x73, 0x75, 0x62, 0x00,
            // pair[3] val: 36 len: 543213c6-1b04-43c1-88aa-2bfe19db8e1b
            0x35, 0x34, 0x33, 0x32, 0x31, 0x33, 0x63, 0x36, 0x2d, 0x31,
            0x62, 0x30, 0x34, 0x2d, 0x34, 0x33, 0x63, 0x31, 0x2d, 0x38,
            0x38, 0x61, 0x61, 0x2d, 0x32, 0x62, 0x66, 0x65, 0x31, 0x39,
            0x64, 0x62, 0x38, 0x65, 0x31, 0x62, 0x00,
            // pair[4] key:  3 len: jti
            0x6a, 0x74, 0x69, 0x00,
            // pair[4] val: 36 len: f4f83d2c-0ac1-4407-a5f6-7048974092bb
            0x66, 0x34, 0x66, 0x38, 0x33, 0x64, 0x32, 0x63, 0x2d, 0x30,
            0x61, 0x63, 0x31, 0x2d, 0x34, 0x34, 0x30, 0x37, 0x2d, 0x61,
            0x35, 0x66, 0x36, 0x2d, 0x37, 0x30, 0x34, 0x38, 0x39, 0x37,
            0x34, 0x30, 0x39, 0x32, 0x62, 0x62, 0x00,
            // pair[5] key:  9 len: auth_time
            0x61, 0x75, 0x74, 0x68, 0x5f, 0x74, 0x69, 0x6d, 0x65, 0x00,
            // pair[5] val:  8 len: f64: 1614961603
            0x00, 0x00, 0xc0, 0xf0, 0x96, 0x10, 0xd8, 0x41, 0x00,
            // pair[6] key: 14 len: email_verified
            0x65, 0x6d, 0x61, 0x69, 0x6c, 0x5f, 0x76, 0x65, 0x72, 0x69,
            0x66, 0x69, 0x65, 0x64, 0x00,
            // pair[6] val:  1 len: false
            0x00, 0x00,
            // pair[7] key:  3 len: azp
            0x61, 0x7a, 0x70, 0x00,
            // pair[7] val:  4 len: test
            0x74, 0x65, 0x73, 0x74, 0x00,
            // pair[8] key: 13 len: session_state
            0x73, 0x65, 0x73, 0x73, 0x69, 0x6f, 0x6e, 0x5f, 0x73, 0x74,
            0x61, 0x74, 0x65, 0x00,
            // pair[8] val: 36 len: f6e37b9c-ce17-4c48-ad94-ee3c8a80a983
            0x66, 0x36, 0x65, 0x33, 0x37, 0x62, 0x39, 0x63, 0x2d, 0x63,
            0x65, 0x31, 0x37, 0x2d, 0x34, 0x63, 0x34, 0x38, 0x2d, 0x61,
            0x64, 0x39, 0x34, 0x2d, 0x65, 0x65, 0x33, 0x63, 0x38, 0x61,
            0x38, 0x30, 0x61, 0x39, 0x38, 0x33, 0x00,
            // pair[9] key:  3 len: iss
            0x69, 0x73, 0x73, 0x00,
            // pair[9] val: 40 len: https://keycloak:8443/auth/realms/master
            0x68, 0x74, 0x74, 0x70, 0x73, 0x3a, 0x2f, 0x2f, 0x6b, 0x65,
            0x79, 0x63, 0x6c, 0x6f, 0x61, 0x6b, 0x3a, 0x38, 0x34, 0x34,
            0x33, 0x2f, 0x61, 0x75, 0x74, 0x68, 0x2f, 0x72, 0x65, 0x61,
            0x6c, 0x6d, 0x73, 0x2f, 0x6d, 0x61, 0x73, 0x74, 0x65, 0x72,
            0x00,
            // pair[10] key:  3 len: acr
            0x61, 0x63, 0x72, 0x00,
            // pair[10] val:  1 len: 1
            0x31, 0x00,
            // pair[11] key: 18 len: preferred_username
            0x70, 0x72, 0x65, 0x66, 0x65, 0x72, 0x72, 0x65, 0x64, 0x5f,
            0x75, 0x73, 0x65, 0x72, 0x6e, 0x61, 0x6d, 0x65, 0x00,
            // pair[11] val:  5 len: admin
            0x61, 0x64, 0x6d, 0x69, 0x6e, 0x00,
            // pair[12] key:  3 len: iat
            0x69, 0x61, 0x74, 0x00,
            // pair[12] val:  8 len: f64: 1614961604
            0x00, 0x00, 0x00, 0xf1, 0x96, 0x10, 0xd8, 0x41, 0x00,
            // pair[13] key:  3 len: typ
            0x74, 0x79, 0x70, 0x00,
            // pair[13] val:  2 len: ID
            0x49, 0x44, 0x00,
        ];
    }

    #[test]
    fn it_parses_filter_metadata() {
        let pairs = Pairs::decode(fixtures::EXAMPLE_METADATA_FILTER_METADATA);
        assert!(pairs.is_ok());
    }

    #[test]
    fn it_parses_envoy_jwt_authn_metadata() {
        let pairs = Pairs::decode(fixtures::EXAMPLE_METADATA_FILTER_ENVOY_JWT_AUTHN);
        assert!(pairs.is_ok());
    }

    #[test]
    fn it_parses_envoy_jwt_authn_metadata_verified_jwt() {
        let pairs = Pairs::decode(fixtures::EXAMPLE_VERIFIED_JWT);
        assert!(pairs.is_ok());
    }
}
