use std::borrow::Cow;
use std::collections::HashMap;

use candid::CandidType;
use jsonrpc_core::{Error, Failure, Id, Version};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;

// A HTTP response.
#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct HttpResponse {
    /// The HTTP status code.
    pub status_code: u16,
    /// The response header map.
    pub headers: HashMap<Cow<'static, str>, Cow<'static, str>>,
    /// The response body.
    pub body: ByteBuf,
    /// Whether the query call should be upgraded to an update call.
    pub upgrade: Option<bool>,
}

impl HttpResponse {
    pub fn new(
        status_code: u16,
        headers: HashMap<Cow<'static, str>, Cow<'static, str>>,
        body: ByteBuf,
        upgrade: Option<bool>,
    ) -> Self {
        Self {
            status_code,
            headers,
            body,
            upgrade,
        }
    }

    pub fn new_failure(
        jsonrpc: Option<Version>,
        id: Id,
        error: Error,
        status_code: HttpStatusCode,
    ) -> Self {
        let failure = Failure { jsonrpc, error, id };
        let body = match serde_json::to_vec(&failure) {
            Ok(bytes) => ByteBuf::from(&bytes[..]),
            Err(e) => ByteBuf::from(e.to_string().as_bytes()),
        };

        Self::new(
            status_code as u16,
            HashMap::from([("content-type".into(), "application/json".into())]),
            body,
            None,
        )
    }

    /// Returns a new `HttpResponse` intended to be used for internal errors.
    pub fn internal_error(e: String) -> Self {
        let body = match serde_json::to_vec(&e) {
            Ok(bytes) => ByteBuf::from(&bytes[..]),
            Err(e) => ByteBuf::from(e.to_string().as_bytes()),
        };

        Self {
            status_code: 500,
            headers: HashMap::from([("content-type".into(), "application/json".into())]),
            body,
            upgrade: None,
        }
    }

    /// Returns an OK response with the given body.
    pub fn ok(body: ByteBuf) -> Self {
        Self::new(
            HttpStatusCode::Ok as u16,
            HashMap::from([("content-type".into(), "application/json".into())]),
            body,
            None,
        )
    }

    /// Upgrade response to update call.
    pub fn upgrade_response() -> Self {
        Self::new(204, HashMap::default(), ByteBuf::default(), Some(true))
    }
}

/// The important components of an HTTP request.
#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct HttpRequest {
    /// The HTTP method string.
    pub method: Cow<'static, str>,
    /// The URL that was visited.
    pub url: Cow<'static, str>,
    /// The request headers.
    pub headers: HashMap<Cow<'static, str>, Cow<'static, str>>,
    /// The request body.
    pub body: ByteBuf,
}

impl HttpRequest {
    pub fn new<T: ?Sized + Serialize>(data: &T) -> Self {
        let mut headers = HashMap::new();
        headers.insert("content-type".into(), "application/json".into());
        Self {
            method: "POST".into(),
            url: "".into(),
            headers,
            body: ByteBuf::from(serde_json::to_vec(&data).unwrap()),
        }
    }

    pub fn decode_body<T>(&self) -> Result<T, Box<HttpResponse>>
    where
        T: DeserializeOwned,
    {
        serde_json::from_slice::<T>(&self.body).map_err(|_| {
            Box::new(HttpResponse::new_failure(
                Some(Version::V2),
                Id::Null,
                Error::parse_error(),
                HttpStatusCode::BadRequest,
            ))
        })
    }

    /// Returns an header value by matching the key with a case-insensitive comparison.
    /// As IC HTTP headers are lowercased, this method cost is usually O(1) for matching lowercase inputs, and O(n) in any other case.
    pub fn get_header_ignore_case(&self, header_name: &str) -> Option<&Cow<'static, str>> {
        match self.headers.get(header_name) {
            Some(ip) => Some(ip),
            None => self
                .headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(header_name))
                .map(|(_, v)| v),
        }
    }
}

#[repr(u16)]
pub enum HttpStatusCode {
    Ok = 200,
    BadRequest = 400,
    InternalServerError = 500,
}
