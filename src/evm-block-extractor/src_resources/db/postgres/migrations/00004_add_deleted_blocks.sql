
-----------------------------
-- Begin - DISCARDED_EVM_BLOCK -
-----------------------------

create table DISCARDED_EVM_BLOCK (
    ID char(66) primary key, -- 64 is the length of a H256 in hex, plus 0x
    DATA JSONB,
    REASON TEXT, 
    DISCARDED_AT TIMESTAMPTZ default (now() AT TIME ZONE 'utc')
);

-- End - DISCARDED_EVM_BLOCK -
