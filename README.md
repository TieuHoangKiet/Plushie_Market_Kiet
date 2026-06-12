# Plushie Market

## Project Title

Plushie Market

## Project Description

Plushie Market is a decentralized collectible marketplace built with Soroban on
the Stellar blockchain. The smart contract records plushie ownership, rarity,
fixed-price listings, purchases, and auctions directly on-chain.

Each plushie receives a token value based on its rarity. Common plushies have a
lower token value, while rarer plushies have a higher value. All purchases and
auction bids use a configured Stellar Asset Contract as the payment token.

## Project Vision

Plushie Market provides collectors with a transparent and decentralized way to
own and trade plushie collectibles. On-chain ownership and market activity let
users verify who owns each plushie and complete trades without relying on a
centralized marketplace.

## Key Features

- **Rarity-Based Token Value:** Plushies are classified as Common, Uncommon,
  Rare, Epic, or Legendary.
- **On-Chain Ownership:** Every plushie has a verifiable owner stored in the
  smart contract.
- **Fixed-Price Marketplace:** Owners can list plushies at their rarity-based
  token value for immediate purchase.
- **Stellar Token Payments:** Buyers pay sellers using the configured Stellar
  Asset Contract.
- **On-Chain Auctions:** Owners can create timed auctions with a rarity-based
  minimum bid.
- **Secure Bid Escrow:** The contract holds the highest bid until settlement.
- **Automatic Bid Refunds:** The previous highest bidder is refunded when a
  higher bid is submitted.
- **Permissionless Settlement:** Anyone can finalize an auction after its
  deadline.
- **Owner Authorization:** Sensitive actions require authorization from the
  relevant owner, seller, buyer, bidder, or administrator.

## Rarity Token Values

The contract calculates each plushie's value by multiplying the configured
token unit by its rarity multiplier:

```text
Token Value = Token Unit * Rarity Multiplier
```

| Rarity | Multiplier |
| --- | ---: |
| Common | 10x |
| Uncommon | 25x |
| Rare | 75x |
| Epic | 200x |
| Legendary | 500x |

For a Stellar token with seven decimal places, initialize the contract with a
token unit of `10_000_000`.

## Usage Instructions

1. **Initialize Market:** Configure the market administrator, payment token,
   and token unit.
2. **Create Plushie:** Create a plushie with a name, owner, and rarity.
3. **List for Sale:** The owner lists a plushie at its rarity-based value.
4. **Buy Plushie:** A buyer pays the seller and receives ownership.
5. **Start Auction:** The owner creates a timed auction.
6. **Place Bid:** Users submit bids while the contract escrows the highest bid.
7. **Finalize Auction:** After the deadline, the seller receives the winning
   bid and the winner receives the plushie.
8. **Cancel or Transfer:** Owners can cancel eligible listings and auctions or
   transfer unlisted plushies.

## Smart Contract Functions

```text
initialize(admin, payment_token, token_unit)
create_plushie(creator, name, rarity)
get_plushie(plushie_id)
rarity_price(rarity)
transfer(owner, to, plushie_id)
list_for_sale(seller, plushie_id)
cancel_sale(seller, plushie_id)
buy(buyer, plushie_id)
start_auction(seller, plushie_id, duration_seconds)
bid(bidder, plushie_id, amount)
cancel_auction(seller, plushie_id)
finalize_auction(plushie_id)
get_sale(plushie_id)
get_auction(plushie_id)
```

## Example Workflow

```text
initialize
    -> create_plushie
    -> list_for_sale or start_auction
    -> buy or bid
    -> finalize_auction
```

A plushie must be created before its ID can be used in marketplace, auction, or
ownership functions.

## Build

The contract uses Soroban SDK version `25`.

```bash
cargo build --release --target wasm32v1-none
```

## Technology Stack

- Rust
- Soroban SDK `25`
- Stellar blockchain
- Stellar Asset Contracts
- Soroban persistent and instance storage

## Security

- Users must authorize actions that transfer their tokens.
- Owners and sellers must authorize listing, cancellation, and transfer
  operations.
- Auction bids are held in contract escrow.
- Auctions with bids cannot be canceled.
- Expired auctions can be finalized by anyone.

Before production use, the contract should receive an independent security
audit and include comprehensive automated tests.

## Future Scope

- Add creator royalties and marketplace transaction fees.
- Support multiple payment tokens.
- Add verified rarity authorities or external rarity oracles.
- Link physical plushies using QR codes or NFC tags.
- Add collector profiles, trading history, and rankings.
- Build a web or mobile marketplace interface.

## Contribution

Contributions are welcome from blockchain developers, Soroban smart contract
engineers, designers, and collectible communities.

## License

This project is licensed under the MIT License.

## Contract Detail

Contract ID:

```text
CALBJEYQSTBMQHGR2SQRDZLUN6TVAWCVBCVYJWCJPXJXGO5Y75ALMW3Y
```

Stellar Expert:

```text
https://stellar.expert/explorer/testnet/contract/CALBJEYQSTBMQHGR2SQRDZLUN6TVAWCVBCVYJWCJPXJXGO5Y75ALMW3Y
```
