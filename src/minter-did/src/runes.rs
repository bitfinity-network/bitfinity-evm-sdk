use ordinals::RuneId;

use crate::error::Error;
use crate::id256::Id256;

impl From<RuneId> for Id256 {
    fn from(value: RuneId) -> Self {
        Self::from_btc_tx_index(value.block, value.tx)
    }
}

impl TryFrom<Id256> for RuneId {
    type Error = Error;

    fn try_from(value: Id256) -> Result<Self, Self::Error> {
        let (block, tx) = value.to_btc_tx_index()?;
        Ok(RuneId { block, tx })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_from_rune_id() {
        let rune_id = RuneId { block: 256, tx: 42 };
        let id = Id256::from(rune_id);

        assert_eq!(id.try_into(), Ok(rune_id));
    }
}
