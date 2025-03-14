use alloy::rpc::json_rpc::ErrorPayload;

pub mod params;
pub mod request;
pub mod response;

/// Returns an `ErrorPayload` with a generic parse error message.
pub fn invalid_params_with_details<T: AsRef<str>>(message: T) -> ErrorPayload {
    use std::borrow::Cow;

    ErrorPayload {
        message: Cow::Owned(format!("Invalid params: {}", message.as_ref())),
        ..ErrorPayload::invalid_params()
    }
}
