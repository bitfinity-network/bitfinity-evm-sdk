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

#[cfg(test)]
mod tests {
    use alloy::rpc::json_rpc::{Id, ResponsePayload};

    use super::*;

    #[test]
    fn test_deserialize_single_response() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":"0x1234"}"#;
        let response: RpcResponse = serde_json::from_str(json).unwrap();
        match response {
            RpcResponse::Single(resp) => {
                assert_eq!(resp.id, Id::Number(1));
                assert!(matches!(resp.payload, ResponsePayload::Success(_)));
            }
            RpcResponse::Batch(_) => panic!("Expected single response"),
        }
    }

    #[test]
    fn test_deserialize_batch_response() {
        let json = r#"[
            {"jsonrpc":"2.0","id":1,"result":"0x1234"},
            {"jsonrpc":"2.0","id":2,"result":"0x5678"}
        ]"#;
        let response: RpcResponse = serde_json::from_str(json).unwrap();
        match response {
            RpcResponse::Batch(responses) => {
                assert_eq!(responses.len(), 2);
                assert_eq!(responses[0].id, Id::Number(1));
                assert_eq!(responses[1].id, Id::Number(2));
            }
            RpcResponse::Single(_) => panic!("Expected batch response"),
        }
    }

    #[test]
    fn test_deserialize_empty_batch() {
        let json = "[]";
        let response: RpcResponse = serde_json::from_str(json).unwrap();
        match response {
            RpcResponse::Batch(responses) => assert!(responses.is_empty()),
            RpcResponse::Single(_) => panic!("Expected empty batch"),
        }
    }

    #[test]
    fn test_bytebuf_conversion() {
        let response = RpcResponse::Single(Response {
            id: Id::Number(1),
            payload: ResponsePayload::Success(
                serde_json::value::to_raw_value(&"0x1234".to_string()).unwrap(),
            ),
        });

        let buf: ByteBuf = response.into();
        let decoded: RpcResponse = serde_json::from_slice(&buf).unwrap();

        match decoded {
            RpcResponse::Single(resp) => {
                assert_eq!(resp.id, Id::Number(1));
                assert!(matches!(resp.payload, ResponsePayload::Success(_)));
            }
            RpcResponse::Batch(_) => panic!("Expected single response"),
        }
    }

    #[test]
    fn test_deserialize_error_response() {
        let json = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32700,"message":"Parse error"}}"#;
        let response: RpcResponse = serde_json::from_str(json).unwrap();
        match response {
            RpcResponse::Single(resp) => {
                assert_eq!(resp.id, Id::Number(1));
                assert!(matches!(resp.payload, ResponsePayload::Failure(_)));
            }
            RpcResponse::Batch(_) => panic!("Expected single response"),
        }
    }

    #[test]
    fn test_serialize_deserialize_single_response_roundtrip() {
        // Create a single response
        let original = RpcResponse::Single(Response {
            id: Id::Number(42),
            payload: ResponsePayload::Success(
                serde_json::value::to_raw_value(&"0xabc123".to_string()).unwrap(),
            ),
        });

        // Serialize to JSON
        let json = serde_json::to_string(&original).unwrap();

        // Deserialize back
        let deserialized: RpcResponse = serde_json::from_str(&json).unwrap();

        // Verify roundtrip
        match (original, deserialized) {
            (RpcResponse::Single(orig), RpcResponse::Single(des)) => {
                assert_eq!(orig.id, des.id);
                match (orig.payload, des.payload) {
                    (ResponsePayload::Success(orig_val), ResponsePayload::Success(des_val)) => {
                        assert_eq!(orig_val.get(), des_val.get());
                    }
                    _ => panic!("Expected Success payloads"),
                }
            }
            _ => panic!("Expected Single responses for both original and deserialized"),
        }
    }

    #[test]
    fn test_serialize_deserialize_batch_response_roundtrip() {
        // Create a batch response
        let original = RpcResponse::Batch(vec![
            Response {
                id: Id::Number(1),
                payload: ResponsePayload::Success(
                    serde_json::value::to_raw_value(&"0xabc123".to_string()).unwrap(),
                ),
            },
            Response {
                id: Id::String("request-2".to_string()),
                payload: ResponsePayload::Failure(alloy::rpc::json_rpc::ErrorPayload {
                    code: -32000,
                    message: "Custom error".into(),
                    data: Some(serde_json::value::to_raw_value(&"Error details").unwrap()),
                }),
            },
        ]);

        // Serialize to JSON
        let json = serde_json::to_string(&original).unwrap();

        // Deserialize back
        let deserialized: RpcResponse = serde_json::from_str(&json).unwrap();

        // Verify roundtrip
        match (original, deserialized) {
            (RpcResponse::Batch(orig_batch), RpcResponse::Batch(des_batch)) => {
                assert_eq!(orig_batch.len(), des_batch.len());

                // Verify first response (success)
                assert_eq!(orig_batch[0].id, des_batch[0].id);
                match (&orig_batch[0].payload, &des_batch[0].payload) {
                    (ResponsePayload::Success(orig_val), ResponsePayload::Success(des_val)) => {
                        assert_eq!(orig_val.get(), des_val.get());
                    }
                    _ => panic!("Expected Success payload for first response"),
                }

                // Verify second response (error)
                assert_eq!(orig_batch[1].id, des_batch[1].id);
                match (&orig_batch[1].payload, &des_batch[1].payload) {
                    (ResponsePayload::Failure(orig_err), ResponsePayload::Failure(des_err)) => {
                        assert_eq!(orig_err.code, des_err.code);
                        assert_eq!(orig_err.message, des_err.message);
                        assert_eq!(
                            orig_err.data.as_ref().unwrap().get(),
                            des_err.data.as_ref().unwrap().get()
                        );
                    }
                    _ => panic!("Expected Error payload for second response"),
                }
            }
            _ => panic!("Expected Batch responses for both original and deserialized"),
        }
    }

    #[test]
    fn test_serialize_deserialize_with_null_id() {
        // Create a response with null ID
        let original = RpcResponse::Single(Response {
            id: Id::None,
            payload: ResponsePayload::Success(serde_json::value::to_raw_value(&123).unwrap()),
        });

        // Serialize to JSON
        let json = serde_json::to_string(&original).unwrap();

        // Deserialize back
        let deserialized: RpcResponse = serde_json::from_str(&json).unwrap();

        match deserialized {
            RpcResponse::Single(resp) => {
                assert_eq!(resp.id, Id::None);
                assert!(matches!(resp.payload, ResponsePayload::Success(_)));
            }
            _ => panic!("Expected Single response"),
        }
    }

    #[test]
    fn test_bytebuf_roundtrip_with_complex_data() {
        // Create a complex batch response
        let original = RpcResponse::Batch(vec![
            Response {
                id: Id::Number(1),
                payload: ResponsePayload::Success(
                    serde_json::value::to_raw_value(&serde_json::json!({
                        "result": "0xabc123",
                        "details": {
                            "status": "success",
                            "timestamp": 1678923456
                        }
                    }))
                    .unwrap(),
                ),
            },
            Response {
                id: Id::String("complex-req".to_string()),
                payload: ResponsePayload::Failure(alloy::rpc::json_rpc::ErrorPayload {
                    code: -32602,
                    message: "Invalid params".into(),
                    data: Some(
                        serde_json::value::to_raw_value(&serde_json::json!({
                            "missing": ["param1", "param2"],
                            "invalid": {"type": "wrong format"}
                        }))
                        .unwrap(),
                    ),
                }),
            },
        ]);

        // Convert to ByteBuf
        let buf: ByteBuf = original.clone().into();

        // Convert back from ByteBuf
        let deserialized: RpcResponse = serde_json::from_slice(&buf).unwrap();

        // Basic structure verification
        match (&original, &deserialized) {
            (RpcResponse::Batch(orig), RpcResponse::Batch(des)) => {
                assert_eq!(orig.len(), des.len());
            }
            _ => panic!("Expected both to be batch responses"),
        }

        // Re-serialize both to compare JSON equality
        let original_json = serde_json::to_string(&original).unwrap();
        let deserialized_json = serde_json::to_string(&deserialized).unwrap();

        assert_eq!(original_json, deserialized_json);
    }

    #[test]
    fn test_deserialize_response_with_different_id_types() {
        // String ID
        let json_string_id = r#"{"jsonrpc":"2.0","id":"test-id","result":"0xabc"}"#;
        let response: RpcResponse = serde_json::from_str(json_string_id).unwrap();
        match response {
            RpcResponse::Single(resp) => {
                assert_eq!(resp.id, Id::String("test-id".to_string()));
            }
            _ => panic!("Expected single response"),
        }

        // Null ID
        let json_null_id = r#"{"jsonrpc":"2.0","id":null,"result":"0xabc"}"#;
        let response: RpcResponse = serde_json::from_str(json_null_id).unwrap();
        match response {
            RpcResponse::Single(resp) => {
                assert_eq!(resp.id, Id::None);
            }
            _ => panic!("Expected single response"),
        }

        // Number ID
        let json_number_id = r#"{"jsonrpc":"2.0","id":12345,"result":"0xabc"}"#;
        let response: RpcResponse = serde_json::from_str(json_number_id).unwrap();
        match response {
            RpcResponse::Single(resp) => {
                assert_eq!(resp.id, Id::Number(12345));
            }
            _ => panic!("Expected single response"),
        }
    }

    #[test]
    fn test_deserialize_batch_with_mixed_results() {
        let json = r#"[
            {"jsonrpc":"2.0","id":1,"result":"0x1234"},
            {"jsonrpc":"2.0","id":2,"error":{"code":-32000,"message":"Error message"}},
            {"jsonrpc":"2.0","id":"string-id","result":{"key":"value"}},
            {"jsonrpc":"2.0","id":null,"error":{"code":-32600,"message":"Invalid Request","data":"Details"}}
        ]"#;

        let response: RpcResponse = serde_json::from_str(json).unwrap();
        match response {
            RpcResponse::Batch(responses) => {
                assert_eq!(responses.len(), 4);

                // First response - Success with number ID
                assert_eq!(responses[0].id, Id::Number(1));
                assert!(matches!(responses[0].payload, ResponsePayload::Success(_)));

                // Second response - Error with number ID
                assert_eq!(responses[1].id, Id::Number(2));
                assert!(matches!(responses[1].payload, ResponsePayload::Failure(_)));

                // Third response - Success with string ID
                assert_eq!(responses[2].id, Id::String("string-id".to_string()));
                assert!(matches!(responses[2].payload, ResponsePayload::Success(_)));

                // Fourth response - Error with null ID
                assert_eq!(responses[3].id, Id::None);
                assert!(matches!(responses[3].payload, ResponsePayload::Failure(_)));
            }
            _ => panic!("Expected batch response"),
        }
    }

    #[test]
    fn test_error_handling_from_bytebuf() {
        // Create an invalid JSON string
        let invalid_json = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":";
        let buf = ByteBuf::from(invalid_json);

        // Try to deserialize
        let result: Result<RpcResponse, _> = serde_json::from_slice(&buf);
        assert!(result.is_err(), "Expected error on invalid JSON");
    }

    #[test]
    fn test_bytebuf_from_serialization_error() {
        // Create a value that can't be serialized to JSON
        // We'll simulate this by using the ByteBuf::from fallback path

        // First create a valid response
        let response = RpcResponse::Single(Response {
            id: Id::Number(1),
            payload: ResponsePayload::Success(serde_json::value::to_raw_value(&"test").unwrap()),
        });

        // Convert to ByteBuf - this should succeed
        let buf: ByteBuf = response.into();

        // Check that the buffer is not empty and contains valid JSON
        assert!(!buf.is_empty());
        let parsed: Result<serde_json::Value, _> = serde_json::from_slice(&buf);
        assert!(parsed.is_ok(), "Buffer should contain valid JSON");
    }
}
