use std::fmt::Debug;

use thiserror::Error;

pub mod binance;
pub mod bithumb;
pub mod common;
pub mod cryptocom;
mod dynamic;
pub mod okx;
pub mod upbit;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot serialize request body into JSON: {0}")]
    SerializeJsonBody(serde_json::Error),
    #[error("cannot serialize request to URL-encoded parameters: {0}")]
    SerializeUrlencodedBody(serde_urlencoded::ser::Error),
    #[error("cannot serialize request to URL-encoded parameters: {0}")]
    SerializeUrlencodedBodyUpbit(serde_urlencoded_upbit::ser::Error),
    #[error("cannot construct http::Request: {0}")]
    ConstructHttpRequest(nerf::http::Error),
    #[error("cannot deserialize response into JSON: {0}, payload: {1}")]
    DeserializeJsonBody(serde_json::Error, String),
    #[error("request to API server returned error, code: {code:?}, message: {msg:?}")]
    RequestFailed {
        code: Option<String>,
        msg: Option<String>,
    },
    #[error(transparent)]
    Hyper(#[from] hyper::Error),
    #[error("cannot sign JWT payload for authentication: {0}")]
    Jwt(jwt::Error),
    #[error("Unsupported HTTP method {0}")]
    UnsupportedHttpMethod(nerf::http::Method),
    /// A boxed error variant.
    /// [tower::buffer::Buffer] returns a Boxed error type so [Client]s must implement
    /// `From<Box<dyn StdError + Send + Sync + 'static>>` to support buffering.
    ///
    /// The conversion is done by manual downcasting to possible inner error variants
    /// and this variant is a fallback if every downcast fails.
    #[error(transparent)]
    Boxed(Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl From<Box<dyn std::error::Error + Send + Sync + 'static>> for Error {
    fn from(x: Box<dyn std::error::Error + Send + Sync + 'static>) -> Self {
        match x.downcast::<hyper::Error>() {
            Ok(x) => Self::Hyper(*x),
            Err(x) => Self::Boxed(x),
        }
    }
}

#[derive(Clone)]
pub struct KeySecretAuthentication {
    key: String,
    secret: String,
}

impl KeySecretAuthentication {
    pub fn new(key: &str, secret: &str) -> Self {
        Self {
            key: key.to_string(),
            secret: secret.to_string(),
        }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn secret(&self) -> &str {
        &self.secret
    }
}

impl Debug for KeySecretAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeySecretAuthentication")
            .field("key", &Box::new("<redacted>"))
            .field("secret", &Box::new("redacted"))
            .finish()
    }
}

mod ts_milliseconds_str {
    use chrono::{serde::ts_milliseconds, DateTime, TimeZone, Utc};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let value = s
            .parse::<i64>()
            .map_err(|e| serde::de::Error::custom(e.to_string()))?;
        match Utc.timestamp_opt(value / 1000, ((value % 1000) * 1_000_000) as u32) {
            chrono::LocalResult::Single(x) => Ok(x),
            _ => Err(serde::de::Error::custom(format!(
                "cannot deserialize DateTime from timestamp_millis {value}"
            ))),
        }
    }

    #[allow(dead_code)]
    pub fn serialize<S>(this: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        struct InterceptSerializer<S>(S);

        impl<S: Serializer> Serializer for InterceptSerializer<S> {
            type Ok = S::Ok;

            type Error = S::Error;

            type SerializeSeq = S::SerializeSeq;

            type SerializeTuple = S::SerializeTuple;

            type SerializeTupleStruct = S::SerializeTupleStruct;

            type SerializeTupleVariant = S::SerializeTupleVariant;

            type SerializeMap = S::SerializeMap;

            type SerializeStruct = S::SerializeStruct;

            type SerializeStructVariant = S::SerializeStructVariant;

            fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
                self.0.serialize_str(&format!("{v}"))
            }

            fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
                self.0.serialize_str(&format!("{v}"))
            }

            fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
                self.0.serialize_str(&format!("{v}"))
            }

            fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
                self.0.serialize_str(&format!("{v}"))
            }

            fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
                self.0.serialize_str(&format!("{v}"))
            }

            fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
                self.0.serialize_str(&format!("{v}"))
            }

            fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
                self.0.serialize_str(&format!("{v}"))
            }

            fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
                self.0.serialize_str(&format!("{v}"))
            }

            fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
                self.0.serialize_str(&format!("{v}"))
            }

            fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
                self.0.serialize_f32(v)
            }

            fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
                self.0.serialize_f64(v)
            }

            fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
                self.0.serialize_char(v)
            }

            fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
                self.0.serialize_str(v)
            }

            fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
                self.0.serialize_bytes(v)
            }

            fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
                self.0.serialize_none()
            }

            fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
            where
                T: Serialize,
            {
                self.0.serialize_some(value)
            }

            fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
                self.0.serialize_unit()
            }

            fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
                self.0.serialize_unit_struct(name)
            }

            fn serialize_unit_variant(
                self,
                _name: &'static str,
                _variant_index: u32,
                _variant: &'static str,
            ) -> Result<Self::Ok, Self::Error> {
                todo!()
            }

            fn serialize_newtype_struct<T: ?Sized>(
                self,
                name: &'static str,
                value: &T,
            ) -> Result<Self::Ok, Self::Error>
            where
                T: Serialize,
            {
                self.0.serialize_newtype_struct(name, value)
            }

            fn serialize_newtype_variant<T: ?Sized>(
                self,
                name: &'static str,
                variant_index: u32,
                variant: &'static str,
                value: &T,
            ) -> Result<Self::Ok, Self::Error>
            where
                T: Serialize,
            {
                self.0
                    .serialize_newtype_variant(name, variant_index, variant, value)
            }

            fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
                self.0.serialize_seq(len)
            }

            fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
                self.0.serialize_tuple(len)
            }

            fn serialize_tuple_struct(
                self,
                name: &'static str,
                len: usize,
            ) -> Result<Self::SerializeTupleStruct, Self::Error> {
                self.0.serialize_tuple_struct(name, len)
            }

            fn serialize_tuple_variant(
                self,
                name: &'static str,
                variant_index: u32,
                variant: &'static str,
                len: usize,
            ) -> Result<Self::SerializeTupleVariant, Self::Error> {
                self.0
                    .serialize_tuple_variant(name, variant_index, variant, len)
            }

            fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
                self.0.serialize_map(len)
            }

            fn serialize_struct(
                self,
                name: &'static str,
                len: usize,
            ) -> Result<Self::SerializeStruct, Self::Error> {
                self.0.serialize_struct(name, len)
            }

            fn serialize_struct_variant(
                self,
                name: &'static str,
                variant_index: u32,
                variant: &'static str,
                len: usize,
            ) -> Result<Self::SerializeStructVariant, Self::Error> {
                self.0
                    .serialize_struct_variant(name, variant_index, variant, len)
            }
        }

        let iserializer = InterceptSerializer(serializer);
        ts_milliseconds::serialize(this, iserializer)
    }
}
