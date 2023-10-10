use std::rc::Rc;

use anyhow::Context;
use itertools::Itertools;
use jsonrpc_core::{Call, Id, MethodCall, Output, Params, Request, Response, Version};
use serde::Deserialize;

use crate::MAX_BATCH_REQUESTS;

async fn send_rpc_request(url: &str, request: Request) -> anyhow::Result<Response> {
    log::trace!("sending request {request:?}");

    let response = reqwest::Client::new()
        .post(url)
        .json(&request)
        .send()
        .await
        .context("failed to send RPC request")?
        .json::<Response>()
        .await
        .context("failed to decode RPC response")?;

    log::trace!("response: {:?}", response);

    Ok(response)
}

pub async fn single_request<R>(
    url: &str,
    method: String,
    params: Params,
    id: Id,
) -> anyhow::Result<R>
where
    R: for<'a> Deserialize<'a>,
{
    let request = Request::Single(Call::MethodCall(MethodCall {
        jsonrpc: Some(Version::V2),
        method,
        params,
        id,
    }));

    let response = send_rpc_request(url, request).await?;

    match response {
        Response::Single(response) => match response {
            Output::Success(result) => {
                serde_json::from_value(result.result).context("failed to deserialize value")
            }
            Output::Failure(err) => Err(anyhow::format_err!("{err:?}")),
        },
        Response::Batch(_) => Err(anyhow::format_err!("unexpected response type: batch")),
    }
}

/// The idea of the `batch request` method is that different requests are supposed to be independent so we return
/// a signle result for every call. But sometimes we can receive an error that is related to some subset of the requests.
/// In this case we need to replicate that error several times. But `anyhow::Error` does not implement `Clone` that
/// why we need to use `Rc<anyhow::Error>`.
type BatchEntityResult<T> = std::result::Result<T, Rc<anyhow::Error>>;

pub async fn batch_request<R>(
    url: &str,
    method: String,
    params: impl IntoIterator<Item = (Params, Id)>,
) -> Vec<BatchEntityResult<R>>
where
    R: for<'a> Deserialize<'a>,
{
    let mut results = Vec::new();

    let append_errors = |results: &mut Vec<BatchEntityResult<R>>, error: anyhow::Error, count| {
        let error = Rc::new(error);
        for _ in 0..count {
            results.push(Err(error.clone()))
        }
    };

    let append_json_value = |results: &mut Vec<BatchEntityResult<R>>, value| {
        results
            .push(serde_json::from_value::<R>(value).map_err(|e| Rc::new(anyhow::Error::from(e))));
    };

    for chunk in &params.into_iter().chunks(MAX_BATCH_REQUESTS) {
        let method_calls = chunk
            .map(|(params, id)| {
                Call::MethodCall(MethodCall {
                    jsonrpc: Some(Version::V2),
                    method: method.clone(),
                    params,
                    id,
                })
            })
            .collect::<Vec<_>>();
        let chunk_size = method_calls.len();
        let request = Request::Batch(method_calls);

        let response = send_rpc_request(url, request).await;

        match response {
            Err(err) => append_errors(&mut results, anyhow::format_err!("{err:?}"), chunk_size),
            Ok(Response::Single(response)) => match response {
                Output::Success(result) => {
                    if chunk_size == 1 {
                        append_json_value(&mut results, result.result);
                    } else {
                        append_errors(&mut results, anyhow::format_err!("invalid"), chunk_size);
                    }
                }
                Output::Failure(err) => {
                    append_errors(&mut results, anyhow::format_err!("{err:?}"), chunk_size);
                }
            },
            Ok(Response::Batch(response)) => {
                if chunk_size == response.len() {
                    for resp in response.into_iter() {
                        match resp {
                            Output::Success(resp) => append_json_value(&mut results, resp.result),
                            Output::Failure(err) => {
                                append_errors(&mut results, anyhow::format_err!("{err:?}"), 1)
                            }
                        }
                    }
                } else {
                    append_errors(&mut results, anyhow::format_err!("invalid"), chunk_size);
                }
            }
        }
    }

    results
}
