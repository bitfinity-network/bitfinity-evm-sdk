pub mod error;
pub mod reservation;

use std::time::Duration;

pub use error::{Error, Result};
pub use reservation::ReservationService;

pub trait TimeWaiter {
    /// Wait for the given duration
    fn wait(&self, duration: Duration) -> impl std::future::Future<Output = ()> + Send;
}

#[cfg(feature = "tokio")]
pub mod tokio_waiter {
    use std::time::Duration;

    /// A time waiter that uses tokio's sleep function
    pub struct TokioTimeWaiter;

    impl Default for TokioTimeWaiter {
        fn default() -> Self {
            Self::new()
        }
    }

    impl TokioTimeWaiter {
        pub fn new() -> Self {
            Self
        }
    }

    impl super::TimeWaiter for TokioTimeWaiter {
        async fn wait(&self, duration: Duration) {
            tokio::time::sleep(duration).await;
        }
    }
}
