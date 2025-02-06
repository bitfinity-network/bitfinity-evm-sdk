
-----------------------------
-- Begin - DISCARDED_EVM_BLOCK -
-----------------------------

create table DISCARDED_EVM_BLOCK (
    ID bigint primary key,
    DATA JSONB,
    DISCARDED_AT TIMESTAMP,
    REASON TEXT 
);

-- End - DISCARDED_EVM_BLOCK -

-----------------------------------------
-- Begin - DISCARDED_EVM_TRANSACTION -
-----------------------------------------

create table DISCARDED_EVM_TRANSACTION (
    ID char(66) primary key, -- 64 is the length of a H256 in hex, plus 0x
    DATA JSONB,
    BLOCK_NUMBER bigint
);

CREATE INDEX DISCARDED_EVM_TRANSACTION_INDEX_BLOCK_NUMBER ON DISCARDED_EVM_TRANSACTION( BLOCK_NUMBER );

-- End - DISCARDED_EVM_TRANSACTION -

