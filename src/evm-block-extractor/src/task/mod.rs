use std::fmt::Debug;
use std::future::Future;
use std::time::Duration;

pub mod block_extractor;

/// Tries to execute the future returned by `func` until success up to `retry_count`.
pub async fn with_retry<Res, Err, Fut, Func>(
    description: &str,
    request_delay: Duration,
    retry_count: u32,
    func: Func,
) -> Result<Res, Err>
where
    Fut: Future<Output = Result<Res, Err>>,
    Func: Fn() -> Fut,
    Err: Debug,
{
    for _ in 0..retry_count - 1 {
        match func().await {
            Ok(result) => return Ok(result),
            Err(err) => {
                log::warn!("operation '{description}' failed with error '{err:?}', retrying after {request_delay:?}");
                tokio::time::sleep(request_delay).await
            }
        }
    }

    func().await
}

#[cfg(test)]
mod tests {

    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;

    #[tokio::test]
    async fn retry_should_return_on_success() {
        let val = Rc::new(RefCell::new(0));
        let result = with_retry("test func", Duration::from_secs(1), 5, || async {
            *val.borrow_mut() += 1;

            anyhow::Result::<i32>::Ok(*val.borrow())
        })
        .await;

        assert!(matches!(result, Ok(1)));
    }

    #[tokio::test]
    async fn retry_should_return_on_success_on_several_attempt() {
        let val = Rc::new(RefCell::new(0));
        let result = with_retry("test func", Duration::from_secs(0), 5, || async {
            *val.borrow_mut() += 1;

            if *val.borrow() < 4 {
                Err(anyhow::format_err!("error"))
            } else {
                Ok(*val.borrow())
            }
        })
        .await;

        assert!(matches!(result, Ok(4)));
    }

    #[tokio::test]
    async fn retry_should_return_error_if_all_attempts_fail() {
        let val = Rc::new(RefCell::new(0));
        let result = with_retry("test func", Duration::from_secs(0), 5, || async {
            *val.borrow_mut() += 1;

            anyhow::Result::<()>::Err(anyhow::format_err!("error {}", *val.borrow()))
        })
        .await;

        let _expected_error = anyhow::format_err!("error {}", 5);
        assert!(matches!(result, Err(_expected_error)));
    }
}
