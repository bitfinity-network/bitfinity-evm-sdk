use std::cell::RefCell;

use ic_stable_structures::{StableBTreeMap, StableMultimap, StableVec};

use crate::error::{Error, Result};
use crate::state::{PairKey, LATEST_TIME_MEMORY_ID, PAIR_MEMORY_ID, PRICE_MEMORY_ID};

/// Map of cryptocurrency pairs -> timestamp -> price, like (eth_usdt(), Time, Price);
#[derive(Default)]
pub struct PairPrice {}

impl PairPrice {
    /// Remove all cryptocurrency pairs.
    pub fn reset(&mut self) {
        PRICE_MAP.with(|price| price.borrow_mut().clear());
        LATEST_TIME_MAP
            .with(|time: &RefCell<StableBTreeMap<PairKey, u64>>| time.borrow_mut().clear());
        PAIR_VEC.with(|pairs| pairs.borrow_mut().clear().unwrap());
    }

    /// Returns the all types of price pairs
    pub fn get_pairs(&self) -> Vec<PairKey> {
        PAIR_VEC.with(|pairs| pairs.borrow().iter().collect())
    }

    /// Returns the latest (timestamp, price) of given pair
    pub fn get_latest_price(&self, pair: &PairKey) -> Option<(u64, u64)> {
        let latest_timestamp = LATEST_TIME_MAP.with(|time| time.borrow().get(pair))?;
        let price = PRICE_MAP.with(|price| price.borrow().get(pair, &latest_timestamp))?;
        Some((latest_timestamp, price))
    }

    /// Return the latest n records of a price pair, or latest len() records if the price's amount fewer
    pub fn get_prices(&self, pair: &PairKey, n: usize) -> Vec<(u64, u64)> {
        let prices =
            PRICE_MAP.with(|price| price.borrow().range(pair).collect::<Vec<(u64, u64)>>());
        prices.into_iter().rev().take(n).collect()
    }

    /// Returns whether the given pairkey exists
    pub fn is_exist(&self, pair: &PairKey) -> bool {
        PAIR_VEC.with(|vec| vec.borrow().iter().any(|i| i == *pair))
    }

    /// Add pair to the oracle canister, need to check permission in external function
    /// If pair already exists, returns Error::PairExist.
    pub fn add_pair(&mut self, pair: PairKey) -> Result<()> {
        if pair.0.as_bytes().len() > 16 {
            return Err(Error::PairKeyTooLong(pair.0.as_bytes().len() as u64));
        }
        if PAIR_VEC.with(|pairs| pairs.borrow().iter().any(|i| i == pair)) {
            return Err(Error::PairExist);
        }
        PAIR_VEC.with(|pairs| pairs.borrow_mut().push(&pair))?;
        Ok(())
    }

    /// Delete pair from the oracle canister, need to check permission in external function
    /// If pair doesn't exists, returns Error::PairNotExist.
    pub fn del_pair(&mut self, pair: PairKey) -> Result<()> {
        let len = PAIR_VEC.with(|pairs| pairs.borrow().len());

        if let Some(idx) = PAIR_VEC.with(|pairs| pairs.borrow().iter().position(|x| x == pair)) {
            PAIR_VEC.with(|pairs| {
                let last_key = pairs.borrow().get(len - 1).unwrap();
                pairs.borrow_mut().set(idx as u64, &last_key).unwrap();
                pairs.borrow_mut().pop();

                PRICE_MAP.with(|price| price.borrow_mut().remove_partial(&pair));
                LATEST_TIME_MAP.with(|time| time.borrow_mut().remove(&pair));
                Ok(())
            })
        } else {
            Err(Error::PairNotExist)
        }
    }

    /// update the new price
    pub fn update_price(&mut self, pair: PairKey, timestamp: u64, price: u64) -> Result<()> {
        if !PAIR_VEC.with(|vec| vec.borrow().iter().any(|i| i == pair)) {
            return Err(Error::PairNotExist);
        }

        PRICE_MAP.with(|map| map.borrow_mut().insert(&pair, &timestamp, &price));

        LATEST_TIME_MAP.with(|map| map.borrow_mut().insert(pair, timestamp));

        Ok(())
    }
}

thread_local! {
    static PRICE_MAP: RefCell<StableMultimap<PairKey, u64, u64>> = {
        RefCell::new(StableMultimap::new(PRICE_MEMORY_ID))
    };

    static LATEST_TIME_MAP: RefCell<StableBTreeMap<PairKey, u64>> = {
        RefCell::new(StableBTreeMap::new(LATEST_TIME_MEMORY_ID))
    };

    static PAIR_VEC: RefCell<StableVec<PairKey>> = {
        RefCell::new(StableVec::new(PAIR_MEMORY_ID).unwrap())
    };
}

#[cfg(test)]
mod tests {
    use ic_exports::ic_kit::MockContext;

    use super::PairPrice;
    use crate::error::Error;
    use crate::state::PairKey;

    fn eth_usdt() -> PairKey {
        PairKey("ETHUSDT".to_string())
    }

    fn btc_usdt() -> PairKey {
        PairKey("BTCUSDT".to_string())
    }

    fn icp_usdt() -> PairKey {
        PairKey("ICPUSDT".to_string())
    }

    fn new_pairs() -> PairPrice {
        MockContext::new().inject();
        let mut pairs = PairPrice::default();
        pairs.reset();
        pairs
    }

    fn fill_pairs(pairs: &mut PairPrice) {
        pairs.add_pair(eth_usdt()).unwrap();
        pairs.add_pair(btc_usdt()).unwrap();
        pairs.add_pair(icp_usdt()).unwrap();

        pairs.update_price(eth_usdt(), 0, 1800).unwrap();
        pairs.update_price(eth_usdt(), 1, 1900).unwrap();
        pairs.update_price(eth_usdt(), 2, 2000).unwrap();

        pairs.update_price(btc_usdt(), 1, 28000).unwrap();
        pairs.update_price(btc_usdt(), 2, 29000).unwrap();
        pairs.update_price(btc_usdt(), 3, 30000).unwrap();

        pairs.update_price(icp_usdt(), 2, 5).unwrap();
        pairs.update_price(icp_usdt(), 3, 10).unwrap();
        pairs.update_price(icp_usdt(), 4, 15).unwrap();
    }

    #[test]
    fn reset_should_clear_pairs() {
        let mut pairs = new_pairs();
        fill_pairs(&mut pairs);
        pairs.reset();

        assert!(pairs.get_pairs().is_empty());
        assert!(pairs.get_latest_price(&eth_usdt()).is_none());
        assert!(pairs.get_latest_price(&btc_usdt()).is_none());
        assert!(pairs.get_latest_price(&icp_usdt()).is_none());

        assert!(pairs.get_prices(&eth_usdt(), 10).is_empty());
        assert!(pairs.get_prices(&btc_usdt(), 10).is_empty());
        assert!(pairs.get_prices(&icp_usdt(), 10).is_empty());
    }

    #[test]
    fn add_pair_should_be_available() {
        let mut pairs = new_pairs();
        fill_pairs(&mut pairs);

        assert_eq!(pairs.get_pairs(), vec![eth_usdt(), btc_usdt(), icp_usdt()]);
    }

    #[test]
    fn cant_add_pair_twice() {
        let mut pairs = new_pairs();
        fill_pairs(&mut pairs);

        assert_eq!(pairs.add_pair(eth_usdt()).unwrap_err(), Error::PairExist);
    }

    #[test]
    fn cant_add_too_long_pair() {
        let mut pairs = new_pairs();

        assert_eq!(
            pairs
                .add_pair(PairKey("12345678901234567".to_string()))
                .unwrap_err(),
            Error::PairKeyTooLong(17)
        );
    }

    #[test]
    fn del_pair_should_be_available() {
        let mut pairs = new_pairs();
        fill_pairs(&mut pairs);

        pairs.del_pair(eth_usdt()).unwrap();

        assert!(pairs.get_latest_price(&eth_usdt()).is_none());
        assert_eq!(pairs.get_pairs(), vec![icp_usdt(), btc_usdt()]);
        assert_eq!(pairs.get_prices(&eth_usdt(), 10), vec![]);
        assert_eq!(
            pairs.get_prices(&btc_usdt(), 10),
            vec![(3, 30000), (2, 29000), (1, 28000)]
        );
        assert_eq!(
            pairs.get_prices(&icp_usdt(), 10),
            vec![(4, 15), (3, 10), (2, 5)]
        );

        assert!(pairs.get_latest_price(&eth_usdt()).is_none());
        assert_eq!(pairs.get_latest_price(&btc_usdt()), Some((3, 30000)));
        assert_eq!(pairs.get_latest_price(&icp_usdt()), Some((4, 15)));

        assert_eq!(
            pairs.update_price(eth_usdt(), 4, 2100).unwrap_err(),
            Error::PairNotExist
        );

        assert_eq!(pairs.del_pair(eth_usdt()).unwrap_err(), Error::PairNotExist);
    }

    #[test]
    fn get_latest_price_should_be_available() {
        let mut pairs = new_pairs();
        fill_pairs(&mut pairs);

        assert_eq!(pairs.get_latest_price(&eth_usdt()), Some((2, 2000)));
        assert_eq!(pairs.get_latest_price(&btc_usdt()), Some((3, 30000)));
        assert_eq!(pairs.get_latest_price(&icp_usdt()), Some((4, 15)));
    }

    #[test]
    fn get_prices_should_be_available() {
        let mut pairs = new_pairs();
        fill_pairs(&mut pairs);

        assert_eq!(pairs.get_prices(&eth_usdt(), 1), vec![(2, 2000)]);
        assert_eq!(pairs.get_prices(&eth_usdt(), 2), vec![(2, 2000), (1, 1900)]);
        assert_eq!(
            pairs.get_prices(&eth_usdt(), 3),
            vec![(2, 2000), (1, 1900), (0, 1800)]
        );
        assert_eq!(
            pairs.get_prices(&eth_usdt(), 10),
            vec![(2, 2000), (1, 1900), (0, 1800)]
        );
    }

    #[test]
    fn update_price_should_be_available() {
        let mut pairs = new_pairs();
        fill_pairs(&mut pairs);

        pairs.update_price(eth_usdt(), 4, 2100).unwrap();
        assert_eq!(pairs.get_latest_price(&eth_usdt()), Some((4, 2100)));
        assert_eq!(
            pairs
                .update_price(PairKey("pair".to_string()), 0, 1)
                .unwrap_err(),
            Error::PairNotExist
        );
    }
}
