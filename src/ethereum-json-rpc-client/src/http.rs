use std::{borrow::Cow, collections::HashMap};

use anyhow::Context;
use candid::CandidType;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;


/// The important components of an HTTP request.
#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub struct HttpRequest<'a> {
    /// The HTTP method string.
    pub method: Cow<'a, str>,
    /// The URL method string.
    pub url: Cow<'a, str>,
    /// The request headers.
    pub headers: HashMap<Cow<'a, str>, Cow<'a, str>>,
    /// The request body.
    pub body: ByteBuf,
}

impl HttpRequest<'static> {
    pub fn new<T: ?Sized + Serialize>(data: &T) -> anyhow::Result<Self> {
        let mut headers = HashMap::new();
        headers.insert("content-type".into(), "application/json".into());
        Ok(Self {
            method: "POST".into(),
            headers,
            url: "".into(),
            body: ByteBuf::from(
                serde_json::to_vec(data).context("failed to serialize RPC request")?,
            ),
        })
    }
}

#[derive(Clone, Debug, CandidType, Serialize, Deserialize, PartialEq, Eq)]
pub struct HttpResponse {
    /// The HTTP status code.
    pub status_code: u16,
    /// The response header map.
    pub headers: HashMap<String, String>,
    /// The response body.
    pub body: ByteBuf,
}