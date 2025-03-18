use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::error::Error;
use super::request::Request;

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
    pub fn parse<D>(self) -> Result<D, Error>
    where
        D: DeserializeOwned,
    {
        let value: Value = self.into();
        serde_json::from_value(value).map_err(|e| Error::invalid_params(e.to_string()))
    }
}

/// Default parameters
pub fn default_params() -> Params {
    Params::None
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

pub trait ParamsAccessors {
    /// Get a required parameter from the params array.
    fn get_from_vec<T: DeserializeOwned>(&self, index: usize) -> Result<T, Error>;

    /// Get an optional parameter from the params object.
    fn get_from_object<T: DeserializeOwned>(&self, field: &str) -> Result<Option<T>, Error>;

    /// Checks the type and the number of parameters.
    ///
    /// Fails if
    /// - params is not an array
    /// - number of params is less than `len(req_params)`
    /// - number of params is greater than `max`
    fn validate_params(&self, required_params: &[&str], max_size: usize) -> Result<(), Error>;

    /// Get a required parameter from the params object.
    fn required_from_object<T: DeserializeOwned>(&self, field: &str) -> Result<T, Error> {
        self.get_from_object(field)?
            .ok_or_else(|| Error::invalid_params(format!("missing field '{field}'")))
    }
}

impl ParamsAccessors for Request {
    fn get_from_vec<T: DeserializeOwned>(&self, index: usize) -> Result<T, Error> {
        let Params::Array(params) = &self.params else {
            return Err(Error::invalid_params("missing params"));
        };

        match params.get(index) {
            Some(value) => serde_json::from_value(value.clone()).map_err(|e| {
                Error::invalid_params(format!("failed to deserialize value at index {index}: {e}"))
            }),
            None => Err(Error::invalid_params(format!(
                "index {} exceeds length of params {}",
                index,
                params.len()
            ))),
        }
    }

    fn get_from_object<T: DeserializeOwned>(&self, field: &str) -> Result<Option<T>, Error> {
        let Params::Map(params) = &self.params else {
            return Err(Error::invalid_params(
                "missing params object or params is not an object",
            ));
        };

        match params.get(field) {
            Some(value) if value.is_null() => Ok(None),
            Some(value) => serde_json::from_value(value.clone()).map_err(|e| {
                Error::invalid_params(format!("failed to deserialize value at field {field}: {e}"))
            }),
            None => Ok(None),
        }
    }

    fn validate_params(&self, required_params: &[&str], max_size: usize) -> Result<(), Error> {
        let Params::Array(params) = &self.params else {
            return Err(Error::invalid_params(format!(
                "expected 'params' array of at least {} arguments",
                required_params.len()
            )));
        };

        let param_count = params.len();

        if param_count < required_params.len() {
            return Err(Error::invalid_params(format!(
                "expected at least {} argument/s but received {}: required parameters [{}]",
                required_params.len(),
                param_count,
                required_params.join(", ")
            )));
        }
        if param_count > max_size {
            return Err(Error::invalid_params(format!(
                "too many arguments, want at most {max_size}"
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use alloy::hex;
    use alloy::primitives::{Address, Bytes, B256, B512, U256, U64};
    use proptest::collection::vec;
    use proptest::prelude::*;
    use serde_json;
    use serde_json::{json, Value};

    use super::{Params, *};
    use crate::rpc::error::ErrorCode;
    use crate::rpc::id::Id;
    use crate::rpc::version::Version;
    use crate::BlockNumber;

    fn get_method_call_array(params: Vec<Value>) -> Request {
        Request {
            params: Params::Array(params),
            jsonrpc: Some(Version::V2),
            method: "test_method".into(),
            id: Id::Number(1),
        }
    }

    fn get_method_call_object(params: serde_json::Map<String, Value>) -> Request {
        Request {
            params: Params::Map(params),
            jsonrpc: Some(Version::V2),
            method: "test_method".into(),
            id: Id::Number(1),
        }
    }

    // Testing the get_from_vec function for the following types:
    // Address      | Address, H64, B256, B512 | e.g. invalid address, valid address
    proptest! {
        #[test]
        fn test_get_from_vec_h160(
            address20 in prop::array::uniform20(any::<u8>()),
            hex_address in "[0-9a-fA-F]{40}"
        ) {
            let address = Address::from_slice(&address20);
            let params = vec![json!(address)];
            let method = get_method_call_array(params);
            let result = method.get_from_vec::<Address>(0).unwrap();
            assert_eq!(result, address);
            let hex_address = Address::from_str(&hex_address).unwrap();
            let params = vec![json!(hex_address)];
            let method = get_method_call_array(params);
            let result = method.get_from_vec::<Address>(0).unwrap();
            assert_eq!(result, hex_address);
        }

        #[test]
        fn test_get_from_vec_h512(
            address64 in vec(any::<u8>(), 64),
            hex_address in "[0-9a-fA-F]{128}"
        ) {
            let address = B512::from_slice(&address64);
            let params = vec![json!(address)];
            let method = get_method_call_array(params);
            let result = method.get_from_vec::<B512>(0).unwrap();
            assert_eq!(result, address);
            let hex_address = B512::from_str(&hex_address).unwrap();
            let params = vec![json!(hex_address)];
            let method = get_method_call_array(params);
            let result = method.get_from_vec::<B512>(0).unwrap();
            assert_eq!(result, hex_address);
        }

        #[test]
        fn test_get_from_vec_h256(
            address32 in prop::array::uniform32(any::<u8>()),
            hex_address in "[0-9a-fA-F]{64}"
        ) {
            let address = B256::from_slice(&address32);
            let params = vec![json!(address)];
            let method = get_method_call_array(params);
            let result = method.get_from_vec::<B256>(0).unwrap();
            assert_eq!(result, address);
            let hex_address = B256::from_str(&hex_address).unwrap();
            let params = vec![json!(hex_address)];
            let method = get_method_call_array(params);
            let result = method.get_from_vec::<B256>(0).unwrap();
            assert_eq!(result, hex_address);
        }

        // Get from the Wrong index
        #[test]
        fn test_get_from_vec_address_wrong_index(
            address64 in vec(any::<u8>(), 64),
            index in 1..64usize
        ) {
            let address = B512::from_slice(&address64);
            let params = vec![json!(address)];
            let method = get_method_call_array(params);
            if index == 0 {
                let result = method.get_from_vec::<B512>(index).unwrap();
                assert_eq!(result, address);
            } else {
                let result = method.get_from_vec::<B512>(index);
                assert!(result.is_err());
            }
        }

        #[test]
        fn test_get_from_vec_address_wrong_type(
            address64 in vec(any::<u8>(), 64),
            index in 0..64usize
        ) {
            let address = B512::from_slice(&address64);
            let params = vec![json!(address)];
            let method = get_method_call_array(params);
            let result = method.get_from_vec::<Address>(index);
            assert!(result.is_err());
        }

    }

    // Testing the get_from_vec function for the following types:
    // Bytes        | Bytes | e.g. invalid bytes, valid bytes
    proptest! {
        #[test]
        fn test_get_from_vec_bytes(
            bytes in vec(any::<u8>(), 64),
            bytes_hex in "[0-9a-fA-F]{128}"
        ) {
            let bytes = Bytes::from(bytes);
            let params = vec![json!(bytes)];
            let method = get_method_call_array(params);
            let result = method.get_from_vec::<Bytes>(0).unwrap();
            assert_eq!(result, bytes);
            let bytes_hex = Bytes::from(hex::decode(bytes_hex).unwrap());
            let params = vec![json!(bytes_hex)];
            let method = get_method_call_array(params);
            let result = method.get_from_vec::<Bytes>(0).unwrap();
            assert_eq!(result, bytes_hex);
        }

        #[test]
        fn test_get_from_vec_bytes_wrong_index(
            bytes in vec(any::<u8>(), 64),
            index in 1..64usize
        ) {
            let bytes = Bytes::from(bytes);
            let params = vec![json!(bytes)];
            let method = get_method_call_array(params);
            if index == 0 {
                let result = method.get_from_vec::<Bytes>(index).unwrap();
                assert_eq!(result, bytes);
            } else {
                let result = method.get_from_vec::<Bytes>(index);
                assert!(result.is_err());
            }
        }

        #[test]
        fn test_get_from_vec_bytes_wrong_type(
            bytes in vec(any::<u8>(), 64),
            index in 0..64usize
        ) {
            let bytes = Bytes::from(bytes);
            let params = vec![json!(bytes)];
            let method = get_method_call_array(params);
            let result = method.get_from_vec::<Address>(index);
            assert!(result.is_err());
        }

    }

    // Testing the get_from_vec function for the following types:
    // Quantity     | U256, U64 | e.g. invalid quantity, valid quantity
    proptest! {
        #[test]
        fn test_get_from_vec_u256(
            quantity in any::<u128>()
        ) {
            let quantity = U256::from(quantity);
            let params = vec![json!(quantity)];
            let method = get_method_call_array(params);
            let result = method.get_from_vec::<U256>(0).unwrap();
            assert_eq!(result, quantity);
        }

        #[test]
        fn test_get_from_vec_u64(
            quantity in any::<u64>()
        ) {
            let quantity = U64::from(quantity);
            let params = vec![json!(quantity)];
            let method = get_method_call_array(params);
            let result = method.get_from_vec::<U64>(0).unwrap();
            assert_eq!(result, quantity);
        }

        #[test]
        fn test_get_from_vec_u256_wrong_index(
            quantity in any::<u64>(),
            index in 1..64usize
        ) {
            let quantity = U256::from(quantity);
            let params = vec![json!(quantity)];
            let method = get_method_call_array(params);
            if index == 0 {
                let result = method.get_from_vec::<U256>(index).unwrap();
                assert_eq!(result, quantity);
            } else {
                let result = method.get_from_vec::<U256>(index);
                assert!(result.is_err());
            }
        }

        #[test]
        fn test_get_from_vec_u256_wrong_type(
            quantity in any::<u64>(),
            index in 0..64usize
        ) {
            let quantity = U256::from(quantity);
            let params = vec![json!(quantity)];
            let method = get_method_call_array(params);
            let result = method.get_from_vec::<Address>(index);
            assert!(result.is_err());
        }
    }

    // Testing the get_from_vec function for the following types:
    // Boolean      | bool | e.g. invalid boolean, valid boolean
    proptest! {
        #[test]
        fn test_get_from_vec_bool(
            boolean in any::<bool>(),
        ) {
            let params = vec![json!(boolean)];
            let method = get_method_call_array(params);
            let result = method.get_from_vec::<bool>(0).unwrap();
            assert_eq!(result, boolean);
        }
    }

    // Block_Number | "earliest", "latest", "pending" or U256 |
    // e.g. invalid block number, valid block number
    proptest! {
        #[test]
        fn test_get_from_block_number(
            block_number in any::<usize>(),
        ) {
            let block_number_eth = U64::from(block_number);
            let params = vec![json!(block_number_eth)];
            let method = get_method_call_array(params);
            let result = method.get_from_vec::<BlockNumber>(0).unwrap();
            assert_eq!(result, BlockNumber::Number(block_number.into()));
        }

        #[test]
        fn test_get_from_block_number_string(
            hex_number in any::<u64>(),
        ) {
            let params = vec![json!(format!("0x{hex_number:x}"))];
            let method = get_method_call_array(params);
            let result = method.get_from_vec::<BlockNumber>(0);
            assert!(result.is_ok());
        }
        #[test]
        fn test_get_from_block_number_string_invalid(
          string in prop_oneof![Just("earliest1".to_string()), Just("latest1".to_string()), Just("pending1".to_string())],
            ) {
            let params = vec![json!(string)];
            let method = get_method_call_array(params);
            let result = method.get_from_vec::<BlockNumber>(0);
            assert!(result.is_err());
        }
        #[test]
        fn test_get_from_block_tags(
            string in prop_oneof![Just("earliest".to_string()), Just("latest".to_string()), Just("pending".to_string())],
        ) {
            let params = vec![json!(string)];
            let method = get_method_call_array(params);
            let result = method.get_from_vec::<BlockNumber>(0).unwrap();
            match string.as_str() {
                "earliest" => assert_eq!(result, BlockNumber::Earliest),
                "latest" => assert_eq!(result, BlockNumber::Latest),
                "pending" => assert_eq!(result, BlockNumber::Pending),
                _ => unreachable!(),
            }
        }

    }

    // fn get_from_object<T: DeserializeOwned>(&self, field: &str) -> Result<Option<T>, Error> {
    // Testing the get_from_object function for the following types:
    // Address      | Address | e.g. invalid address, valid address
    // Quantity     | U256, U64 | e.g. invalid quantity, valid quantity
    // Boolean      | bool | e.g. invalid boolean, valid boolean
    // Data         | Bytes | e.g. invalid data, valid data
    // String       | String | e.g. invalid string, valid string
    // Array        | Vec<T> | e.g. invalid array, valid array

    proptest! {
        // Get from present field
        #[test]
        fn test_get_from_object_address(
            address in "[a-f0-9]{40}"
        ) {
            let address = format!("0x{address}");
            let params = json!({
                "address": address,
                "other": "other",
            });
            let params = params.as_object().unwrap();
            let method = get_method_call_object(params.clone());
            let result = method.get_from_object::<Address>("address").unwrap();
            assert_eq!(result, Some(Address::from_str(&address).unwrap()));
        }

        // Get from present field, required
        #[test]
        fn test_get_from_object_address_required(
            address in "[a-f0-9]{40}"
        ) {
            let address = format!("0x{address}");
            let params = json!({
                "address": address,
                "other": "other",
            });
            let params = params.as_object().unwrap();
            let method = get_method_call_object(params.clone());
            let result = method.required_from_object::<Address>("address").unwrap();
            assert_eq!(result, Address::from_str(&address).unwrap());
        }

        // Get from null field, required
        #[test]
        fn test_get_from_object_address_non_required_null(
            _address in "[a-f0-9]{40}"
        ) {
            let params = json!({
                "address": null,
                "other": "other",
            });
            let params = params.as_object().unwrap();
            let method = get_method_call_object(params.clone());
            let result = method.get_from_object::<Address>("address");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), None);
        }

        // Get from null field, required
        #[test]
        fn test_get_from_object_address_required_null(
            _address in "[a-f0-9]{40}"
        ) {
            let params = json!({
                "address": null,
                "other": "other",
            });
            let params = params.as_object().unwrap();
            let method = get_method_call_object(params.clone());
            let result = method.required_from_object::<Address>("address");
            assert!(result.is_err());
        }

    }

    #[test]
    fn test_get_from_object_non_present_non_required() {
        let params = json!({
            "other": "other",
        });

        let params = params.as_object().unwrap();
        let method = get_method_call_object(params.clone());
        let result = method.get_from_object::<Address>("address");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    // Get from non-present field, required
    #[test]
    fn test_get_from_object_non_present_required() {
        let params = json!({
            "other": "other",
        });

        let params = params.as_object().unwrap();
        let method = get_method_call_object(params.clone());
        let result = method.required_from_object::<Address>("address");
        assert!(result.is_err());
    }

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
        let v1: Result<(Option<u8>, String), Error> = params().parse();
        let v2: Result<(u8, bool, String), Error> = params().parse();
        let err1 = v1.unwrap_err();
        let err2 = v2.unwrap_err();

        // then
        assert_eq!(err1.code, ErrorCode::InvalidParams);
        assert_eq!(
            err1.message,
            "invalid type: boolean `true`, expected a string"
        );
        assert!(err1.data.is_none());
        assert_eq!(err2.code, ErrorCode::InvalidParams);
        assert_eq!(
            err2.message,
            "invalid length 2, expected a tuple of size 3"
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
