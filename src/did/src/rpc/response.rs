use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use serde_json::Value;

use super::error::{Error, ErrorCode};
use super::id::Id;
use super::version::Version;

/// Successful response
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Success {
    /// Protocol version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jsonrpc: Option<Version>,
    /// Result
    pub result: Value,
    /// Correlation id
    pub id: Id,
}

/// Unsuccessful response
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Failure {
    /// Protocol Version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jsonrpc: Option<Version>,
    /// Error
    pub error: Error,
    /// Correlation id
    pub id: Id,
}

/// Represents output - failure or success
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
pub enum Response {
    /// Success
    Success(Success),
    /// Failure
    Failure(Failure),
}

impl Response {
    /// Creates new failure Response indicating malformed request.
    pub fn invalid_request(id: Id, jsonrpc: Option<Version>) -> Self {
        Response::Failure(Failure {
            id,
            jsonrpc,
            error: Error::new(ErrorCode::InvalidRequest),
        })
    }

    /// Get the jsonrpc protocol version.
    pub fn version(&self) -> Option<Version> {
        match *self {
            Response::Success(ref s) => s.jsonrpc,
            Response::Failure(ref f) => f.jsonrpc,
        }
    }

    /// Get the correlation id.
    pub fn id(&self) -> &Id {
        match *self {
            Response::Success(ref s) => &s.id,
            Response::Failure(ref f) => &f.id,
        }
    }
}

/// Synchronous response
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
pub enum RpcResponse {
    /// Single response
    Single(Response),
    /// Response to batch request (batch of responses)
    Batch(Vec<Response>),
}

impl RpcResponse {
    /// Creates new `Response` with given error and `Version`
    pub fn from(error: Error, jsonrpc: Option<Version>) -> Self {
        Failure {
            id: Id::Null,
            jsonrpc,
            error,
        }
        .into()
    }

    /// Deserialize `Response` from given JSON string.
    ///
    /// This method will handle an empty string as empty batch response.
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        if s.is_empty() {
            Ok(RpcResponse::Batch(vec![]))
        } else {
            serde_json::from_str::<Self>(s)
        }
    }
}

impl From<Failure> for RpcResponse {
    fn from(failure: Failure) -> Self {
        RpcResponse::Single(Response::Failure(failure))
    }
}

impl From<Success> for RpcResponse {
    fn from(success: Success) -> Self {
        RpcResponse::Single(Response::Success(success))
    }
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

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_deserialize_single_response() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":"0x1234"}"#;
        let response: RpcResponse = serde_json::from_str(json).unwrap();
        match response {
            RpcResponse::Single(resp) => {
                assert_eq!(resp.id(), &Id::Number(1));
                assert!(matches!(resp, Response::Success(_)));
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
                assert_eq!(responses[0].id(), &Id::Number(1));
                assert_eq!(responses[1].id(), &Id::Number(2));
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
        let response = RpcResponse::Single(Response::Success(Success {
            jsonrpc: Some(Version::V2),
            result: "0x1234".into(),
            id: Id::Number(1),
        }));

        let buf: ByteBuf = response.into();
        let decoded: RpcResponse = serde_json::from_slice(&buf).unwrap();

        match decoded {
            RpcResponse::Single(resp) => {
                assert_eq!(resp.id(), &Id::Number(1));
                assert!(matches!(resp, Response::Success(_)));
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
                assert_eq!(resp.id(), &Id::Number(1));
                assert!(matches!(resp, Response::Failure(_)));
            }
            RpcResponse::Batch(_) => panic!("Expected single response"),
        }
    }

    #[test]
    fn test_serialize_deserialize_single_response_roundtrip() {
        // Create a single response
        let original = RpcResponse::Single(Response::Success(Success {
            jsonrpc: Some(Version::V2),
            result: serde_json::json!("0xabc123"),
            id: Id::Number(42),
        }));

        // Serialize to JSON
        let json = serde_json::to_string(&original).unwrap();

        // Deserialize back
        let deserialized: RpcResponse = serde_json::from_str(&json).unwrap();

        // Verify roundtrip
        match (original, deserialized) {
            (RpcResponse::Single(orig), RpcResponse::Single(des)) => {
                assert_eq!(orig.id(), des.id());
                match (orig, des) {
                    (Response::Success(orig_val), Response::Success(des_val)) => {
                        assert_eq!(orig_val, des_val);
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
            Response::Success(Success {
                jsonrpc: Some(Version::V2),
                id: Id::Number(1),
                result: serde_json::json!("0xabc123"),
            }),
            Response::Failure(Failure {
                jsonrpc: Some(Version::V2),
                id: Id::String("request-2".to_string()),
                error: Error {
                    code: ErrorCode::ServerError(-32000),
                    message: "Custom error".into(),
                    data: Some(serde_json::json!("Error details")),
                },
            }),
        ]);
        // Serialize to JSON
        let json = serde_json::to_string(&original).unwrap();

        // Deserialize back
        let deserialized: RpcResponse = serde_json::from_str(&json).unwrap();

        // Verify roundtrip
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_serialize_deserialize_with_null_id() {
        // Create a response with null ID
        let original = RpcResponse::Single(Response::Success(Success {
            jsonrpc: Some(Version::V2),
            result: serde_json::json!(123),
            id: Id::Null,
        }));

        // Serialize to JSON
        let json = serde_json::to_string(&original).unwrap();

        // Deserialize back
        let deserialized: RpcResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_bytebuf_roundtrip_with_complex_data() {
        // Create a complex batch response
        let original = RpcResponse::Batch(vec![
            Response::Success(Success {
                jsonrpc: Some(Version::V2),
                id: Id::Number(1),
                result: serde_json::json!({
                    "result": "0xabc123",
                    "details": {
                        "status": "success",
                        "timestamp": 1678923456
                    }
                }),
            }),
            Response::Failure(Failure {
                jsonrpc: Some(Version::V2),
                id: Id::String("complex-req".to_string()),
                error: Error {
                    code: ErrorCode::InvalidParams,
                    message: "Invalid params".into(),
                    data: Some(serde_json::json!({
                        "missing": ["param1", "param2"],
                        "invalid": {"type": "wrong format"}
                    })),
                },
            }),
        ]);

        // Convert to ByteBuf
        let buf: ByteBuf = original.clone().into();

        // Convert back from ByteBuf
        let deserialized: RpcResponse = serde_json::from_slice(&buf).unwrap();

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
                assert_eq!(resp.id(), &Id::String("test-id".to_string()));
            }
            _ => panic!("Expected single response"),
        }

        // Null ID
        let json_null_id = r#"{"jsonrpc":"2.0","id":null,"result":"0xabc"}"#;
        let response: RpcResponse = serde_json::from_str(json_null_id).unwrap();
        match response {
            RpcResponse::Single(resp) => {
                assert_eq!(resp.id(), &Id::Null);
            }
            _ => panic!("Expected single response"),
        }

        // Number ID
        let json_number_id = r#"{"jsonrpc":"2.0","id":12345,"result":"0xabc"}"#;
        let response: RpcResponse = serde_json::from_str(json_number_id).unwrap();
        match response {
            RpcResponse::Single(resp) => {
                assert_eq!(resp.id(), &Id::Number(12345));
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
                assert_eq!(responses[0].id(), &Id::Number(1));
                assert!(matches!(responses[0], Response::Success(_)));

                // Second response - Error with number ID
                assert_eq!(responses[1].id(), &Id::Number(2));
                assert!(matches!(responses[1], Response::Failure(_)));

                // Third response - Success with string ID
                assert_eq!(responses[2].id(), &Id::String("string-id".to_string()));
                assert!(matches!(responses[2], Response::Success(_)));

                // Fourth response - Error with null ID
                assert_eq!(responses[3].id(), &Id::Null);
                assert!(matches!(responses[3], Response::Failure(_)));
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
        let response = RpcResponse::Single(Response::Success(Success {
            jsonrpc: Some(Version::V2),
            result: serde_json::json!("test"),
            id: Id::Number(1),
        }));

        let buf: ByteBuf = response.into();

        assert!(!buf.is_empty());
        let parsed: Result<serde_json::Value, _> = serde_json::from_slice(&buf);
        assert!(parsed.is_ok(), "Buffer should contain valid JSON");
    }
}
