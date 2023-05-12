mod http_outcall;

pub use http_outcall::{sync_price, transform};

#[cfg(target_arch = "wasm32")]
pub mod wasm32 {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::Duration;

    use ic_exports::ic_cdk;
    use ic_exports::ic_cdk_timers::set_timer_interval;
    use ic_exports::ic_kit::ic;

    use crate::state::{PairKey, PairPrice};
    use crate::timer::sync_price;

    pub fn init_timer(pair_price: Rc<RefCell<PairPrice>>) {
        set_timer_interval(Duration::from_secs(10), move || {
            let pairs: Vec<PairKey> = pair_price
                .clone()
                .borrow()
                .get_pairs()
                .iter()
                .cloned()
                .collect();

            ic_cdk::spawn(async move {
                for pair_key in pairs {
                    let now = ic::time();
                    let res = sync_price(pair_key, now, pair_price.clone()).await;
                    ic_cdk::print(format!("{:?}", res));
                }
            })
        });
    }
}
