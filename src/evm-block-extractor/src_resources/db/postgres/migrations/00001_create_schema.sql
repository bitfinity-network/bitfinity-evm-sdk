
-----------------------------
-- Begin - EVM_BLOCK -
-----------------------------

create table EVM_BLOCK (
    ID bigint primary key,
    DATA JSONB
);

-- End - EVM_BLOCK -


-----------------------------------------
-- Begin - EVM_TRANSACTION_EXE_RESULT -
-----------------------------------------

create table EVM_TRANSACTION_EXE_RESULT (
    ID char(66) primary key, -- 64 is the length of a H256 in hex, plus 0x
    DATA JSONB
);

-- End - EVM_TRANSACTION_EXE_RESULT -


-----------------------------------------
-- Begin - EVM_TRANSACTION -
-----------------------------------------

create table EVM_TRANSACTION (
    ID char(66) primary key, -- 64 is the length of a H256 in hex, plus 0x
    DATA JSONB,
    BLOCK_NUMBER bigint
);

CREATE INDEX EVM_TRANSACTION_INDEX_BLOCK_NUMBER ON EVM_TRANSACTION( BLOCK_NUMBER );

-- End - EVM_TRANSACTION -
