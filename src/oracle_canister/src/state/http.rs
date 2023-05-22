use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Debug;

use candid::{CandidType, Deserialize, Func};
use serde_bytes::ByteBuf;

use crate::state::PairKey;
use crate::state::PairPrice;

#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct Token {}

#[derive(Clone, Debug, CandidType, Deserialize)]
pub enum StreamingStrategy {
    Callback { callback: Func, token: Token },
}

#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct StreamingCallbackHttpResponse {
    pub body: ByteBuf,
    pub token: Option<Token>,
}

/// The important components of an HTTP request.
#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct HttpRequest {
    /// The HTTP method string.
    pub method: Cow<'static, str>,
    /// The URL that was visited.
    pub url: String,
    /// The request headers.
    pub headers: HashMap<Cow<'static, str>, Cow<'static, str>>,
    /// The request body.
    pub body: ByteBuf,
}

/// A HTTP response.
#[derive(Clone, Debug, CandidType)]
pub struct HttpResponse {
    /// The HTTP status code.
    pub status_code: u16,
    /// The response header map.
    pub headers: HashMap<&'static str, &'static str>,
    /// The response body.
    pub body: ByteBuf,
    /// The strategy for streaming the rest of the data, if the full response is to be streamed.
    pub streaming_strategy: Option<StreamingStrategy>,
    /// Whether the query call should be upgraded to an update call.
    pub upgrade: Option<bool>,
}

impl HttpResponse {
    pub fn new(
        status_code: u16,
        headers: HashMap<&'static str, &'static str>,
        body: ByteBuf,
        streaming_strategy: Option<StreamingStrategy>,
        upgrade: Option<bool>,
    ) -> Self {
        Self {
            status_code,
            headers,
            body,
            streaming_strategy,
            upgrade,
        }
    }
}

pub fn http(req: HttpRequest, now: u64, pair_price: &PairPrice) -> HttpResponse {
    match req.method.as_ref() {
        "GET" => {
            let mut s = String::new();
            s.push_str("The following will display the latest 100 pieces of data of each pair in the format of (timestamp in nano sec, price * 1_0000_0000):\n\n");
            s.push_str(&format!("now: {now}\n\n"));

            let pairs: Vec<PairKey> = pair_price.get_pairs().to_vec();

            for pair in pairs {
                let prices = pair_price.get_prices(&pair, 100);
                s.push_str(&coin_name_to_symbol(pair.0));
                s.push('\n');
                s.push_str(&format!("{prices:?}"));
                s.push_str("\n\n");
            }

            HttpResponse::new(
                200,
                HashMap::from([("content-type", "text/plain")]),
                ByteBuf::from(s.as_bytes()),
                None,
                None,
            )
        }
        _ => HttpResponse {
            status_code: 400,
            headers: HashMap::new(),
            body: ByteBuf::from("Bad Request: only supports GET.".as_bytes()),
            streaming_strategy: None,
            upgrade: None,
        },
    }
}

fn coin_name_to_symbol(name: String) -> String {
    match name.as_ref() {
        "bitcoin" => "BTC-USD".to_string(),
        "ethereum" => "ETH-USD".to_string(),
        "internet-computer" => "ICP-USD".to_string(),
        "ordinals" => "ORDI-USD".to_string(),
        "dfuk" => "DFUK-USD".to_string(),
        "pepebrc" => "PEPE(Ordinals)-USD".to_string(),
        "pizabrc" => "PIZA(Ordinals)-USD".to_string(),
        "biso" => "BISO-USD".to_string(),
        "meme-brc-20" => "MEME(Ordinals)-USD".to_string(),
        _ => format!("{}-USD", name),
    }
}
