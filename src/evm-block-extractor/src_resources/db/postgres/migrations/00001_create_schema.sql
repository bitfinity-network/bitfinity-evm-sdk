
-----------------------------
-- Begin - EVM_BLOCK -
-----------------------------

create table EVM_BLOCK (
    ID bigint primary key,
    DATA JSONB
);

-- End - EVM_BLOCK -


-----------------------------------------
-- Begin - EVM_TRANSACTION_RECEIPT -
-----------------------------------------

create table EVM_TRANSACTION_RECEIPT (
    ID char(66) primary key, -- 64 is the length of a H256 in hex, plus 0x
    DATA JSONB
);

-- End - EVM_TRANSACTION_RECEIPT -
