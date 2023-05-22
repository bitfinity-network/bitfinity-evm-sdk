mod http_outcall;

pub use http_outcall::{sync_coinbase_price, sync_coingecko_price, transform};

#[cfg(target_arch = "wasm32")]
pub mod wasm32 {
    use std::time::Duration;

    use ic_exports::ic_cdk;
    use ic_exports::ic_cdk_timers::set_timer_interval;
    use ic_exports::ic_kit::ic;

    use crate::state::{PairKey, PairPrice};
    use crate::timer::sync_coingecko_price;

    pub fn init_timer(mut pair_price: PairPrice) {
        // Every 5 mins will update the price
        set_timer_interval(Duration::from_secs(300), move || {
            let pairs: Vec<PairKey> = pair_price.get_pairs().to_vec();

            ic_cdk::spawn(async move {
                let res = sync_coingecko_price(pairs, &mut pair_price).await;
                ic::print(format!("res: {:?}", res));
            })
        });
    }
}
