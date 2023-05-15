# Example Oracle on ICP

The importance of Oracle to a blockchain system is self-evident, and it is the only way for the blockchain to obtain off-chain information, including real-world asserts/businesses.

## how chainlink works

Chainlink Data Feeds provide data that is aggregated from many data sources by a decentralized set of independent node operators. The Decentralized Data Model describes this in detail. However, there are some exceptions where data for a feed can come only from a single data source or where data values are calculated.

## how oracle works
The solution provided by chainlink has been criticized by centralization, and users need to trust that the nodes of chainlink will not do evil. Because chainlink nodes do not run a consensus algorithm, they do not form an effective blockchain network.

In Internet Computers, we have more decentralized solutions.

### http_outcalls

In IC, canister can initiate http outcalls. That is, smart contracts can directly access off-chain data through the protocol's underly interface: http_request, and ensure security through the entire subnet. How is this possible?

> The HTTPS outcalls feature allows canisters to make outgoing HTTP calls to conventional Web 2.0 HTTP servers. The response of the request can be safely used in computations of the canister, without the risk of state divergence between the replicas of the subnet.

In general, this function is implemented in the subnet node, which is the replica program in the official document. When the nodes receive the http outcalls request initiated by the canister, the nodes will obtain data from Web 2.0 HTTP servers. These data need to go through consensus, that is, all nodes in the subnet need to obtain data from the link specified by canister, and the effective part of the data obtained by more than 2/3 nodes must be completely consistent. Since the data obtained by different nodes may have slight differences, such as timestamps, etc., we need to define a transform function to remove these differences to ensure the consensus through the subnet.

This is our transform function, we only need to get the status and body in the http response, and discard all others such as the HTTP Header:

```rs
pub fn transform(raw: TransformArgs) -> HttpResponse {
    HttpResponse {
        status: raw.response.status,
        body: raw.response.body,
        ..Default::default()
    }
}
```

The price information we need is located in the body, but sometimes the price information obtained by different nodes is different (that is, the price has changed in a short period of time), which means that http outcalls will fail. But don’t worry, generally this kind of The cases are very rare, and even if they happen, we can resend a http outcalls to solve it. Because http outcalls are very fast, generally 8s can get the result, and it is cheap:

> However, note that an HTTP outcall with a small response, like the ones used for querying financial APIs, only costs fractions of a USD cent, which is substantially cheaper than fees charged for a call by oracles on most blockchains.


For more technical details, please refer to [how http_requests works](https://internetcomputer.org/docs/current/developer-docs/integrations/http_requests/http_requests-how-it-works).

### timer

Another component that the oracle canister needs is the `timer`. We need a scheduled task, and every once in a while, such as 10s, we need to obtain data off the chain. This operation is completely done independently by the canister, without any off-chain services participating. As far as I know, Internet Computer is the only blockchain platform whose smart contracts support timers.

With the library provided by dfinity, [ic-cdk-timers](https://github.com/dfinity/cdk-rs/tree/main/src/ic-cdk-timers), we can easily implement a timer every 10s.

```rs
    pub fn init_timer(mut pair_price: PairPrice) {
        // Every 10s will update the price
        set_timer_interval(Duration::from_secs(10), move || {
            let pairs: Vec<PairKey> = pair_price.get_pairs().to_vec();

            ic_cdk::spawn(async move {
                for pair_key in pairs {
                    let now = ic::time();

                    let res = sync_price(pair_key, now, &mut pair_price).await;
                }
            })
        });
    }
```

### http server

Finally, the canister can also be used as an http server. You can directly access the canister in the browser. For example, our demo canister can be accessed through: https://p6xvw-7iaaa-aaaap-aaana-cai.raw.ic0.app/

Accessing smart contracts directly in the browser can eliminate some centralization risks. The frontend of dapps on other blockchains, such as uniswap, is centralized, and there are some acts of [abuse of rights](https://www.coindesk.com/tech/2022/08/22/popular-uniswap-frontend-blocks-over-250-crypto-addresses-related-to-defi-crimes/).

## Summarize

Due to the versatility of http outcalls, we can easily integrate data from coinbase, coinecko, coinmarketcap and so on. For example, for the `brc-20` token that was very popular a while ago, chainlink cannot provide their price-feeding service, but oracle canister can. Moreover, because http outcalls are cheap and have few restrictions, this will cause a revolution in oracle. Oracle are no longer exclusive to high-net-profit services(such as DeFi at the top), but will greatly enhance blockchain's access to the real world and derive many unexpected gameplay.