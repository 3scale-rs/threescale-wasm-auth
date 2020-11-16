use std::str::FromStr;

#[derive(Debug, Copy, Clone)]
pub enum Method {
    GET,
    POST,
    PUT,
    DELETE,
}

impl AsRef<str> for Method {
    fn as_ref(&self) -> &str {
        match self {
            Self::GET => "get",
            Self::POST => "post",
            Self::PUT => "put",
            Self::DELETE => "delete",
        }
    }
}

impl FromStr for Method {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "get" => Self::GET,
            "post" => Self::POST,
            "put" => Self::PUT,
            "delete" => Self::DELETE,
            other => anyhow::bail!("unrecognized HTTP method {}", other),
        })
    }
}

pub struct Endpoint<B, D, T> {
    method: Method,
    path: String,
    headers: Vec<(String, String)>,
    body: Option<B>,
    trailers: Option<Vec<(String, String)>>,
    deserializer: D,
    data_type: core::marker::PhantomData<T>,
}

impl<B, D, T> Endpoint<B, D, T> {
    pub fn method(&self) -> Method {
        self.method
    }

    pub fn path(&self) -> &str {
        self.path.as_str()
    }

    pub fn headers(&self) -> &Vec<(String, String)> {
        self.headers.as_ref()
    }

    pub fn body(&self) -> Option<&B> {
        self.body.as_ref()
    }

    pub fn headers_as_str(&self) -> Vec<(&str, &str)> {
        self.headers()
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }
}

impl<'de, B, D: serde::Deserializer<'de>, T: serde::Deserialize<'de>> Endpoint<B, D, T> {
    pub fn parse(&self, response: &[u8]) -> T {
        //self.deserializer.::<T>(response)
        //self.deserializer.deserialize_any(<T as serde::Deserialize<'de>::>)
    }
}
