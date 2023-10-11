use std::rc::Rc;

use anyhow::Context;
use itertools::Itertools;
use jsonrpc_core::{Call, Id, MethodCall, Output, Params, Request, Response, Version};
use serde::Deserialize;

const MAX_BATCH_REQUESTS: usize = 5; // Max batch size is 5 in EVM

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

pub async fn batch_request<R>(
    url: &str,
    method: String,
    params: impl IntoIterator<Item = (Params, Id)>,
) -> anyhow::Result<Vec<R>>
where
    R: for<'a> Deserialize<'a>,
{
    let mut results = Vec::new();

    let value_from_json = |value| serde_json::from_value::<R>(value);

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

        let response = send_rpc_request(url, request).await?;

        match response {
            Response::Single(response) => match response {
                Output::Success(result) => {
                    if chunk_size == 1 {
                        results.push(value_from_json(result.result)?);
                    } else {
                        anyhow::bail!(
                            "unexpected number of results: have: 1, expected {chunk_size}"
                        );
                    }
                }
                Output::Failure(err) => {
                    anyhow::bail!("{err:?}");
                }
            },
            Response::Batch(response) => {
                if chunk_size == response.len() {
                    for resp in response.into_iter() {
                        match resp {
                            Output::Success(resp) => results.push(value_from_json(resp.result)?),
                            Output::Failure(err) => {
                                anyhow::bail!("{err:?}");
                            }
                        }
                    }
                } else {
                    anyhow::bail!(
                        "unexpected number of results: have: {}, expected {}",
                        response.len(),
                        chunk_size
                    );
                }
            }
        }
    }

    Ok(results)
}
