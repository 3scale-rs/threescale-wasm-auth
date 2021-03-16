use crate::util::pairs::Pairs;
use std::{borrow::Cow, error::Error};
use thiserror::Error;

use crate::configuration::{Decode, Format, LookupType, Operation};
use crate::proxy::metadata::Metadata;

#[derive(Debug, Error)]
pub(crate) enum ValueError {
    #[error("type mismatch, can only decode strings or bytes")]
    Type,
    #[error("error decoding base64")]
    DecodeBase64(#[source] base64::DecodeError),
    #[error("error decoding protobuf")]
    //DecodeProtobuf(#[source] protobuf::ProtobufError),
    DecodeProtobuf(#[source] prost::DecodeError),
    #[error("error decoding JSON")]
    DecodeJSON(#[source] serde_json::Error),
    #[error("error decoding pairs")]
    DecodePairs,
    #[error("multiple errors in or condition")]
    MultipleErrors(Vec<Self>),
    #[error("can only look up objects or lists")]
    LookupMismatch,
}

#[derive(Debug, Clone)]
pub(crate) enum Value {
    Bytes(Vec<u8>),
    String(String),
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

impl Value {
    pub fn to_string(self) -> Option<String> {
        match self {
            Value::String(s) => Some(s),
            Value::Bytes(b) => String::from_utf8(b).ok(),
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

    fn decode(&self, decode: &Decode) -> Result<Value, ValueError> {
        // convert to bytes - decode always operates on string or bytes values, so this should work in a well formed pipeline of ops
        let bytes = match self.as_bytes() {
            Some(bytes) => bytes,
            None => return Err(ValueError::Type),
        };

        log::debug!("Decoding {} bytes: [", bytes.len());
        bytes
            .chunks(8)
            .map(|c| {
                c.iter()
                    .map(|c| {
                        (format!("0x{:02x}", *c), {
                            let ch = char::from(*c);
                            if ch.is_ascii_graphic() {
                                ch
                            } else {
                                ' '
                            }
                        })
                    })
                    .unzip::<_, _, Vec<_>, String>()
            })
            .map(|(b, s)| format!("{}  | {}", b.join(", "), s))
            .for_each(
                |line| //must call per-line, because there are line-decorators
                log::debug!("{}", line),
            );
        log::debug!("]");

        let res = match decode {
            Decode::Base64Decode => Value::Bytes(
                base64::decode_config(bytes, base64::STANDARD)
                    .map_err(|e| ValueError::DecodeBase64(e))?,
            ),
            Decode::Base64URLDecode => Value::Bytes(
                base64::decode_config(bytes, base64::URL_SAFE)
                    .map_err(|e| ValueError::DecodeBase64(e))?,
            ),
            Decode::ProtobufValue => {
                //let proto = {
                //    let mut cis = protobuf::CodedInputStream::from_bytes(bytes);
                //    cis.read_message::<protobuf::well_known_types::Struct>()
                //};
                //let proto = <prost_types::Struct as prost::Message>::decode(bytes);
                let proto = <Metadata as ::prost::Message>::decode(bytes);

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
                    Err(e) => Err(ValueError::DecodeProtobuf(e))?,
                }
            }
            Decode::JsonValue => {
                let json = serde_json::from_slice::<serde_json::Value>(bytes);
                match json {
                    Ok(value) => Value::JsonValue(value),
                    Err(e) => Err(ValueError::DecodeJSON(e))?,
                }
            }
        };

        Ok(res)
    }

    pub fn perform_op(&self, op: &Operation) -> Result<Value, ValueError> {
        let value = match op {
            Operation::Or(ors) => {
                let mut errors = Vec::new();
                ors.iter()
                    .find_map(|op| match self.perform_op(op) {
                        Ok(v) => Some(v),
                        Err(e) => {
                            //errors.push(format!("{}", e));
                            errors.push(e);
                            None
                        }
                    })
                    .ok_or_else(|| ValueError::MultipleErrors(errors))
            }
            Operation::And(ands) => self.decode_multiple(ands),
            Operation::Decode(d) => self.decode(d),
            Operation::Lookup {
                input,
                kind,
                output,
            } => self.lookup(kind, input, output),
        };

        value
    }

    pub fn lookup(
        &self,
        kind: &LookupType,
        input: &Format,
        output: &Format,
    ) -> Result<Value, ValueError> {
        match self {
            Value::Bytes(_) | Value::String(_) => Err(ValueError::LookupMismatch),
            Value::JsonValue(json) => {
                let val = match kind {
                    LookupType::Position(pos) => {
                        let val = json
                            .as_array()
                            .map(|ary| ary.get(*pos))
                            .flatten()
                            .ok_or_else(|| ValueError::LookupMismatch)?;
                        val.clone()
                    }
                    LookupType::Key(key) => {
                        let val = json
                            .as_object()
                            .map(|obj| obj.get(key))
                            .flatten()
                            .ok_or_else(|| ValueError::LookupMismatch)?;
                        val.clone()
                    }
                };
                //Ok(Value::JsonValue(val))
                let out = match output {
                    Format::String => Value::String(
                        val.as_str()
                            .ok_or_else(|| ValueError::LookupMismatch)?
                            .into(),
                    ),
                    _ => Value::JsonValue(val),
                    //    Format::Array => Value::Array(
                    //        val.as_array()
                    //            .ok_or_else(|| ValueError::LookupMismatch)?,
                    //    ),
                    //    Format::Struct => Value::JsonValue(
                    //        val.as_object()
                    //            .ok_or_else(|| ValueError::LookupMismatch)?,
                    //    ),
                };
                Ok(out)
            }
            _ => unimplemented!(),
        }
    }

    pub fn decode_multiple(&self, ops: &Vec<Operation>) -> Result<Value, ValueError> {
        let op0 = &ops[0];
        let initval = self.perform_op(op0)?;
        let mut tmp = Some(initval);
        for op in &ops[1..] {
            if let Some(val) = tmp {
                match val.perform_op(op) {
                    Ok(newval) => {
                        tmp = Some(newval);
                    }
                    Err(e) => return Err(e),
                }
            }
        }

        match tmp {
            Some(v) => Ok(v),
            _ => Err(ValueError::Type),
        }
    }

    fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Bytes(v) => Some(v.as_slice()),
            Value::String(v) => Some(v.as_bytes()),
            _ => None,
        }
    }
}
