use core::fmt;
use std::marker::PhantomData;

use alloy::rpc::json_rpc::Response;
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_bytes::ByteBuf;

/// Response to a `RpcRequest`. If the request was a `Batch` the response will be a `Batch` as well and vice-versa for `One`
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum RpcResponse {
    Single(Response),
    Batch(Vec<Response>),
}

impl From<RpcResponse> for ByteBuf {
    fn from(value: RpcResponse) -> Self {
        match serde_json::to_vec(&value)
            .map_err(|e| ByteBuf::from(e.to_string().as_bytes()))
            .map(ByteBuf::from)
        {
            Ok(bytes) => bytes,
            Err(e) => e,
        }
    }
}

impl<'de> Deserialize<'de> for RpcResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ResponsePacketVisitor {
            marker: PhantomData<fn() -> RpcResponse>,
        }

        impl<'de> Visitor<'de> for ResponsePacketVisitor {
            type Value = RpcResponse;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a single response or a batch of responses")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut responses = Vec::new();

                while let Some(response) = seq.next_element()? {
                    responses.push(response);
                }

                Ok(RpcResponse::Batch(responses))
            }

            fn visit_map<M>(self, map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let response =
                    Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))?;
                Ok(RpcResponse::Single(response))
            }
        }

        deserializer.deserialize_any(ResponsePacketVisitor {
            marker: PhantomData,
        })
    }
}
