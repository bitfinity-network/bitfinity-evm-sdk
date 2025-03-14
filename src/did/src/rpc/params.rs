use alloy::rpc::json_rpc::ErrorPayload;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::invalid_params_with_details;

/// Request parameters for JSON-RPC calls.
/// This enum covers all common parameter formats for JSON-RPC requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Params {
    /// No parameters
    None,
    /// Array of values
    Array(Vec<Value>),
    /// Map of string keys to values
    Map(serde_json::Map<String, Value>),
}

impl Params {
    /// Create an empty parameter set
    pub fn new_none() -> Self {
        Params::None
    }

    /// Create parameters from a vector of values
    pub fn new_array(values: Vec<Value>) -> Self {
        Params::Array(values)
    }

    /// Create parameters from a map of string keys to values
    pub fn new_map(map: serde_json::Map<String, Value>) -> Self {
        Params::Map(map)
    }

    /// Parse parameters into the expected type
    pub fn parse<D>(self) -> Result<D, ErrorPayload>
    where
        D: DeserializeOwned,
    {
        let value: Value = self.into();
        serde_json::from_value(value).map_err(|e| invalid_params_with_details(e.to_string()))
    }
}

impl From<Params> for Value {
    fn from(params: Params) -> Value {
        match params {
            Params::Array(vec) => Value::Array(vec),
            Params::Map(map) => Value::Object(map),
            Params::None => Value::Null,
        }
    }
}

#[cfg(test)]
mod tests {
    use alloy::rpc::json_rpc::ErrorPayload;
    use serde_json;

    use super::Params;

    #[test]
    fn params_deserialization() {
        let s = r#"[null, true, -1, 4, 2.3, "hello", [0], {"key": "value"}, []]"#;
        let deserialized: Params = serde_json::from_str(s).unwrap();

        let mut map = serde_json::Map::new();
        map.insert(
            "key".to_string(),
            serde_json::Value::String("value".to_string()),
        );

        assert_eq!(
            Params::Array(vec![
                serde_json::Value::Null,
                serde_json::Value::Bool(true),
                serde_json::Value::from(-1),
                serde_json::Value::from(4),
                serde_json::Value::from(2.3),
                serde_json::Value::String("hello".to_string()),
                serde_json::Value::Array(vec![serde_json::Value::from(0)]),
                serde_json::Value::Object(map),
                serde_json::Value::Array(vec![]),
            ]),
            deserialized
        );
    }

    #[test]
    fn should_return_meaningful_error_when_deserialization_fails() {
        // given
        let s = r#"[1, true]"#;
        let params = || serde_json::from_str::<Params>(s).unwrap();

        // when
        let v1: Result<(Option<u8>, String), ErrorPayload> = params().parse();
        let v2: Result<(u8, bool, String), ErrorPayload> = params().parse();
        let err1 = v1.unwrap_err();
        let err2 = v2.unwrap_err();

        // then
        assert_eq!(err1.code, -32602);
        assert_eq!(
            err1.message,
            "Invalid params: invalid type: boolean `true`, expected a string"
        );
        assert!(err1.data.is_none());
        assert_eq!(err2.code, -32602);
        assert_eq!(
            err2.message,
            "Invalid params: invalid length 2, expected a tuple of size 3"
        );
        assert!(err2.data.is_none());
    }

    #[test]
    fn single_param_parsed_as_tuple() {
        let params: (u64,) = Params::Array(vec![serde_json::Value::from(1)])
            .parse()
            .unwrap();
        assert_eq!(params, (1,));
    }
}
