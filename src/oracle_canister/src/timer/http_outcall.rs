use candid::Deserialize;
use ic_exports::ic_cdk::api::management_canister::http_request::{
    http_request, CanisterHttpRequestArgument, HttpHeader, HttpMethod, HttpResponse, TransformArgs,
    TransformContext,
};
use url::Url;

use crate::error::{Error, Result};
use crate::state::{PairKey, PairPrice, PRICE_MULTIPLE};

#[derive(Debug, Default, Deserialize)]
struct ResBody {
    pub symbol: String,
    pub price: String,
}

pub async fn http_outcall(
    url: String,
    method: HttpMethod,
    body: Option<Vec<u8>>,
    max_response_bytes: Option<u64>,
) -> Result<HttpResponse> {
    let real_url = Url::parse(&url).map_err(|e| Error::HttpError(e.to_string()))?;
    let headers = vec![
        HttpHeader {
            name: "Host".to_string(),
            value: real_url
                .domain()
                .ok_or_else(|| Error::HttpError("empty domain of url".to_string()))?
                .to_string(),
        },
        HttpHeader {
            name: "User-Agent".to_string(),
            value: "oracle canister".to_string(),
        },
    ];

    let request = CanisterHttpRequestArgument {
        url,
        max_response_bytes,
        method,
        headers,
        body,
        transform: Some(TransformContext::new(transform, vec![])),
    };

    let res = http_request(request.clone())
        .await
        .map(|(res,)| res)
        .map_err(|(r, m)| Error::HttpError(format!("RejectionCode: {r:?}, Error: {m}")))?;

    Ok(res)
}

pub fn transform(raw: TransformArgs) -> HttpResponse {
    let mut sanitized = raw.response;
    sanitized.headers = vec![
        HttpHeader {
            name: "Content-Security-Policy".to_string(),
            value: "default-src 'self'".to_string(),
        },
        HttpHeader {
            name: "Referrer-Policy".to_string(),
            value: "strict-origin".to_string(),
        },
        HttpHeader {
            name: "Permissions-Policy".to_string(),
            value: "geolocation=(self)".to_string(),
        },
        HttpHeader {
            name: "Strict-Transport-Security".to_string(),
            value: "max-age=63072000".to_string(),
        },
        HttpHeader {
            name: "X-Frame-Options".to_string(),
            value: "DENY".to_string(),
        },
        HttpHeader {
            name: "X-Content-Type-Options".to_string(),
            value: "nosniff".to_string(),
        },
    ];
    sanitized
}

pub async fn sync_price(
    pair_key: PairKey,
    timestamp: u64,
    pair_price: &mut PairPrice,
) -> Result<()> {
    let mut base_url = "https://api.binance.com/api/v3/ticker/price?symbol=".to_string();
    base_url.push_str(&pair_key.0);
    let res = http_outcall(base_url, HttpMethod::GET, None, Some(1024)).await?;

    let json_body = serde_json::from_slice::<ResBody>(&res.body)
        .map_err(|e| Error::HttpError(format!("serde_json err: {e}")))?;

    let price_f64 = json_body.price.parse::<f64>().unwrap();
    let price_u64 = (price_f64 * PRICE_MULTIPLE).round() as u64;
    if json_body.symbol != pair_key.0 {
        return Err(Error::Internal(
            "http response's symbol isn't the pair key".to_string(),
        ));
    }
    pair_price.update_price(pair_key, timestamp, price_u64)?;

    Ok(())
}
