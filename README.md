# Plushie Market

## Project Title
Plushie Market

## Project Description
Plushie Market is a decentralized collectible marketplace built with Soroban on the Stellar blockchain. The smart contract records plushie ownership, rarity, supply, listings, purchases, and auctions directly on-chain.

Each plushie receives a dynamic token value based on its rarity and the number of matching plushies in circulation. Rare plushies have a higher token value, while commonly available plushies have a lower value. Owners can sell their plushies at a fixed price or create auctions where users compete using a configured Stellar token.

## Project Vision
The vision of Plushie Market is to provide collectors with a transparent and decentralized platform for trading digital or physical plushie collectibles. By storing ownership, rarity, supply, prices, and auction results on Stellar, the platform allows users to verify collectible information and trade without relying on a centralized marketplace.

## Key Features
- **Rarity-Based Token Value:** Plushies are classified as Common, Rare, Epic, or Legendary.
- **Supply-Based Token Value:** The token value decreases when more plushies with the same collection and name are minted.
- **On-Chain Ownership:** Every plushie has a verifiable owner stored in the smart contract.
- **Fixed-Price Marketplace:** Owners can list plushies for immediate purchase.
- **Token Payments:** Buyers pay sellers using the configured Stellar Asset Contract.
- **On-Chain Auctions:** Owners can create auctions with a minimum starting price.
- **Secure Bid Escrow:** The smart contract holds the highest bid until the auction is settled.
- **Automatic Bid Refunds:** A previous highest bidder is refunded when a higher bid is submitted.
- **Auction Settlement:** The seller receives the winning bid and ownership transfers to the winner.
- **Public Market Queries:** Anyone can inspect plushies, owners, supply, listings, and token values.

## Token Value Model
The contract calculates the collectible token value using:

```text
Token Value = 100 * Rarity Multiplier / Matching Plushie Supply
```

| Rarity | Multiplier |
| --- | ---: |
| Common | 1x |
| Rare | 3x |
| Epic | 7x |
| Legendary | 15x |

For example, a unique Legendary plushie has a token value of `1,500`. If two matching plushies exist, its value becomes `750`.

## Usage Instructions
1. **Initialize Market:** Configure the market admin and Stellar payment token.
2. **Mint Plushie:** Create a plushie with an owner, name, collection, and rarity.
3. **Check Token Value:** Query the plushie's rarity and supply-based token value.
4. **List at Fixed Price:** The owner lists a plushie for immediate purchase.
5. **Buy Plushie:** A buyer pays the listed price and receives ownership.
6. **Start Auction:** The owner creates an auction with a starting price.
7. **Place Bid:** Users submit token bids while the contract escrows the highest bid.
8. **Settle Auction:** The seller receives the winning bid and the winner receives the plushie.
9. **Cancel or Transfer:** Owners can cancel eligible listings or transfer unlisted plushies.

## Smart Contract Functions

```text
initialize(admin, payment_token)
mint_plushie(owner, name, collection, rarity)
list_fixed(owner, plushie_id, price)
buy(plushie_id, buyer)
start_auction(owner, plushie_id, starting_price)
bid(plushie_id, bidder, amount)
settle(seller, plushie_id)
cancel(owner, plushie_id)
transfer(owner, plushie_id, new_owner)
get_plushie(plushie_id)
owner_of(plushie_id)
all_plushies()
supply_of(collection, name)
token_value(plushie_id)
```

## Example Workflow

```text
initialize
    -> mint_plushie
    -> list_fixed or start_auction
    -> buy or bid
    -> settle
```

A plushie must be minted before its ID can be used with `list_fixed`, `start_auction`, `get_plushie`, or other ownership functions.

## Build and Test
The contract uses Soroban SDK version `25`.

```bash
cargo test
cargo build --release --target wasm32v1-none
```

The unit tests are included directly in `lib.rs` and cover:

- Rarity and supply-based token values.
- Fixed-price purchases and ownership transfers.
- Auction bid escrow and previous bidder refunds.
- Auction settlement.
- Invalid low bids and listing cancellation rules.
- Listing with a valid Stellar public address.

## Technology Stack
- Rust for smart contract development.
- Soroban SDK `25`.
- Stellar blockchain for decentralized ownership and transactions.
- Stellar Asset Contracts for payments, bid escrow, and refunds.
- Soroban persistent storage for plushies, ownership, listings, and supply.

## Security Notice
The current demonstration contract removes signature checks from several owner and admin operations to simplify testing. The `buy` and `bid` functions still require authorization because Stellar Asset Contracts require permission before transferring tokens from a user's account.

Before using this contract in production, authorization checks should be restored for administrative and ownership-sensitive operations.

## Future Scope
- Add auction deadlines and permissionless settlement after expiration.
- Restore complete authorization and role-based access control.
- Add creator royalties and marketplace transaction fees.
- Support multiple Stellar payment tokens.
- Add verified rarity authorities or external rarity oracles.
- Link physical plushies using QR codes or NFC tags.
- Add collector profiles, trading history, and rankings.
- Build a web or mobile marketplace interface.

## Contribution
Contributions are welcome from blockchain developers, Soroban smart contract engineers, designers, and collectible communities. Fork the project and submit pull requests to help improve the marketplace.

## License
This project is licensed under the MIT License.

### Contract Detail
ID: `CDX7LX7JEKTY6FXPLUZFMXOWQHFL545D4CX2Q2PQTPF5UBXNE3TO7A2A`

Stellar Expert:

```text
https://stellar.expert/explorer/testnet/contract/CDX7LX7JEKTY6FXPLUZFMXOWQHFL545D4CX2Q2PQTPF5UBXNE3TO7A2A
```
