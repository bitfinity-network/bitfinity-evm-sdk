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
    pub data: CoinBaseData,
}

#[derive(Debug, Default, Deserialize)]
struct CoinBaseData {
    pub base: String,
    pub currency: String,
    pub amount: String,
}

async fn http_outcall(
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
    HttpResponse {
        status: raw.response.status,
        body: raw.response.body,
        ..Default::default()
    }
}

pub async fn sync_price(
    pair_key: PairKey,
    timestamp: u64,
    pair_price: &mut PairPrice,
) -> Result<()> {
    let mut base_url = "https://api.coinbase.com/v2/prices/".to_string();
    base_url.push_str(&pair_key.0);
    base_url.push_str("/spot");
    let res = http_outcall(base_url, HttpMethod::GET, None, Some(8000)).await?;

    let json_body = serde_json::from_slice::<ResBody>(&res.body)
        .map_err(|e| Error::HttpError(format!("serde_json err: {e}")))?;

    let price_f64 = json_body.data.amount.parse::<f64>().unwrap();
    let price_u64 = (price_f64 * PRICE_MULTIPLE).round() as u64;
    let base_currency = format!("{}-{}", json_body.data.base, json_body.data.currency);
    if base_currency != pair_key.0 {
        return Err(Error::Internal(
            "http response's symbol isn't the pair key".to_string(),
        ));
    }
    pair_price.update_price(pair_key, timestamp, price_u64)?;

    Ok(())
}
