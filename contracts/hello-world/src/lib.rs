#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, token, Address, Env, String,
};

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

#[contracttype]
#[derive(Clone)]
pub struct Plushie {
    pub id: u64,
    pub name: String,
    pub owner: Address,
    pub rarity: Rarity,
    pub token_value: i128,
}

#[contracttype]
#[derive(Clone)]
pub struct Sale {
    pub seller: Address,
    pub price: i128,
}

#[contracttype]
#[derive(Clone)]
pub struct Auction {
    pub seller: Address,
    pub min_bid: i128,
    pub highest_bid: i128,
    pub highest_bidder: Option<Address>,
    pub end_time: u64,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Admin,
    PaymentToken,
    TokenUnit,
    NextId,
    Plushie(u64),
    Sale(u64),
    Auction(u64),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum MarketError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    InvalidAmount = 3,
    PlushieNotFound = 4,
    NotOwner = 5,
    AlreadyListed = 6,
    SaleNotFound = 7,
    AuctionNotFound = 8,
    InvalidDuration = 9,
    AuctionEnded = 10,
    AuctionStillRunning = 11,
    BidTooLow = 12,
    AuctionHasBid = 13,
    CannotBuyOwnPlushie = 14,
    Overflow = 15,
}

#[contract]
pub struct PlushieMarket;

#[contractimpl]
impl PlushieMarket {
    /// Initializes the market with the token used for all payments.
    ///
    /// `token_unit` is one whole token in its smallest unit. For a token with
    /// seven decimals, pass 10_000_000.
    pub fn initialize(
        env: Env,
        admin: Address,
        payment_token: Address,
        token_unit: i128,
    ) -> Result<(), MarketError> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(MarketError::AlreadyInitialized);
        }
        if token_unit <= 0 {
            return Err(MarketError::InvalidAmount);
        }

        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::PaymentToken, &payment_token);
        env.storage()
            .instance()
            .set(&DataKey::TokenUnit, &token_unit);
        env.storage().instance().set(&DataKey::NextId, &1_u64);
        Ok(())
    }

    /// Creates a plushie. Its direct-sale price and auction floor are derived
    /// from rarity: Common 10, Uncommon 25, Rare 75, Epic 200, Legendary 500.
    pub fn create_plushie(
        env: Env,
        creator: Address,
        name: String,
        rarity: Rarity,
    ) -> Result<u64, MarketError> {
        creator.require_auth();
        ensure_initialized(&env)?;

        let id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextId)
            .ok_or(MarketError::NotInitialized)?;
        let next_id = id.checked_add(1).ok_or(MarketError::Overflow)?;
        let token_value = rarity_value(&env, rarity)?;

        let plushie = Plushie {
            id,
            name,
            owner: creator,
            rarity,
            token_value,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Plushie(id), &plushie);
        env.storage().instance().set(&DataKey::NextId, &next_id);
        Ok(id)
    }

    pub fn get_plushie(env: Env, plushie_id: u64) -> Result<Plushie, MarketError> {
        read_plushie(&env, plushie_id)
    }

    pub fn rarity_price(env: Env, rarity: Rarity) -> Result<i128, MarketError> {
        rarity_value(&env, rarity)
    }

    /// Transfers a plushie without payment when it is not listed.
    pub fn transfer(
        env: Env,
        owner: Address,
        to: Address,
        plushie_id: u64,
    ) -> Result<(), MarketError> {
        owner.require_auth();
        ensure_not_listed(&env, plushie_id)?;

        let mut plushie = read_plushie(&env, plushie_id)?;
        require_owner(&plushie, &owner)?;
        plushie.owner = to;
        write_plushie(&env, &plushie);
        Ok(())
    }

    /// Lists a plushie at its rarity-derived token value.
    pub fn list_for_sale(env: Env, seller: Address, plushie_id: u64) -> Result<i128, MarketError> {
        seller.require_auth();
        ensure_not_listed(&env, plushie_id)?;

        let plushie = read_plushie(&env, plushie_id)?;
        require_owner(&plushie, &seller)?;
        let price = plushie.token_value;
        env.storage()
            .persistent()
            .set(&DataKey::Sale(plushie_id), &Sale { seller, price });
        Ok(price)
    }

    pub fn cancel_sale(env: Env, seller: Address, plushie_id: u64) -> Result<(), MarketError> {
        seller.require_auth();
        let sale = read_sale(&env, plushie_id)?;
        if sale.seller != seller {
            return Err(MarketError::NotOwner);
        }
        env.storage()
            .persistent()
            .remove(&DataKey::Sale(plushie_id));
        Ok(())
    }

    pub fn buy(env: Env, buyer: Address, plushie_id: u64) -> Result<(), MarketError> {
        buyer.require_auth();
        let sale = read_sale(&env, plushie_id)?;
        if sale.seller == buyer {
            return Err(MarketError::CannotBuyOwnPlushie);
        }

        let mut plushie = read_plushie(&env, plushie_id)?;
        require_owner(&plushie, &sale.seller)?;

        plushie.owner = buyer.clone();
        write_plushie(&env, &plushie);
        env.storage()
            .persistent()
            .remove(&DataKey::Sale(plushie_id));

        let payment_token = read_payment_token(&env)?;
        token::Client::new(&env, &payment_token).transfer(&buyer, &sale.seller, &sale.price);
        Ok(())
    }

    /// Starts an auction whose minimum bid equals the plushie's rarity value.
    pub fn start_auction(
        env: Env,
        seller: Address,
        plushie_id: u64,
        duration_seconds: u64,
    ) -> Result<Auction, MarketError> {
        seller.require_auth();
        if duration_seconds == 0 {
            return Err(MarketError::InvalidDuration);
        }
        ensure_not_listed(&env, plushie_id)?;

        let plushie = read_plushie(&env, plushie_id)?;
        require_owner(&plushie, &seller)?;
        let end_time = env
            .ledger()
            .timestamp()
            .checked_add(duration_seconds)
            .ok_or(MarketError::Overflow)?;
        let auction = Auction {
            seller,
            min_bid: plushie.token_value,
            highest_bid: 0,
            highest_bidder: None,
            end_time,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Auction(plushie_id), &auction);
        Ok(auction)
    }

    /// Escrows the new bid and immediately refunds the previous highest bid.
    pub fn bid(
        env: Env,
        bidder: Address,
        plushie_id: u64,
        amount: i128,
    ) -> Result<(), MarketError> {
        bidder.require_auth();
        let mut auction = read_auction(&env, plushie_id)?;
        if env.ledger().timestamp() >= auction.end_time {
            return Err(MarketError::AuctionEnded);
        }
        if bidder == auction.seller {
            return Err(MarketError::CannotBuyOwnPlushie);
        }
        if amount < auction.min_bid || amount <= auction.highest_bid {
            return Err(MarketError::BidTooLow);
        }

        let previous_bidder = auction.highest_bidder.clone();
        let previous_bid = auction.highest_bid;
        auction.highest_bidder = Some(bidder.clone());
        auction.highest_bid = amount;
        env.storage()
            .persistent()
            .set(&DataKey::Auction(plushie_id), &auction);

        let payment_token = read_payment_token(&env)?;
        let token_client = token::Client::new(&env, &payment_token);
        let contract_address = env.current_contract_address();
        token_client.transfer(&bidder, &contract_address, &amount);
        if let Some(previous_bidder) = previous_bidder {
            token_client.transfer(&contract_address, &previous_bidder, &previous_bid);
        }
        Ok(())
    }

    /// Cancels an auction only while it has no bids.
    pub fn cancel_auction(env: Env, seller: Address, plushie_id: u64) -> Result<(), MarketError> {
        seller.require_auth();
        let auction = read_auction(&env, plushie_id)?;
        if auction.seller != seller {
            return Err(MarketError::NotOwner);
        }
        if auction.highest_bidder.is_some() {
            return Err(MarketError::AuctionHasBid);
        }
        env.storage()
            .persistent()
            .remove(&DataKey::Auction(plushie_id));
        Ok(())
    }

    /// Finalizes an expired auction. Anyone may call this function.
    pub fn finalize_auction(env: Env, plushie_id: u64) -> Result<Option<Address>, MarketError> {
        let auction = read_auction(&env, plushie_id)?;
        if env.ledger().timestamp() < auction.end_time {
            return Err(MarketError::AuctionStillRunning);
        }

        let mut plushie = read_plushie(&env, plushie_id)?;
        require_owner(&plushie, &auction.seller)?;
        env.storage()
            .persistent()
            .remove(&DataKey::Auction(plushie_id));

        if let Some(winner) = auction.highest_bidder {
            plushie.owner = winner.clone();
            write_plushie(&env, &plushie);

            let payment_token = read_payment_token(&env)?;
            token::Client::new(&env, &payment_token).transfer(
                &env.current_contract_address(),
                &auction.seller,
                &auction.highest_bid,
            );
            Ok(Some(winner))
        } else {
            Ok(None)
        }
    }

    pub fn get_sale(env: Env, plushie_id: u64) -> Result<Sale, MarketError> {
        read_sale(&env, plushie_id)
    }

    pub fn get_auction(env: Env, plushie_id: u64) -> Result<Auction, MarketError> {
        read_auction(&env, plushie_id)
    }
}

fn ensure_initialized(env: &Env) -> Result<(), MarketError> {
    if env.storage().instance().has(&DataKey::Admin) {
        Ok(())
    } else {
        Err(MarketError::NotInitialized)
    }
}

fn rarity_value(env: &Env, rarity: Rarity) -> Result<i128, MarketError> {
    ensure_initialized(env)?;
    let unit: i128 = env
        .storage()
        .instance()
        .get(&DataKey::TokenUnit)
        .ok_or(MarketError::NotInitialized)?;
    let multiplier: i128 = match rarity {
        Rarity::Common => 10,
        Rarity::Uncommon => 25,
        Rarity::Rare => 75,
        Rarity::Epic => 200,
        Rarity::Legendary => 500,
    };
    unit.checked_mul(multiplier).ok_or(MarketError::Overflow)
}

fn read_payment_token(env: &Env) -> Result<Address, MarketError> {
    ensure_initialized(env)?;
    env.storage()
        .instance()
        .get(&DataKey::PaymentToken)
        .ok_or(MarketError::NotInitialized)
}

fn read_plushie(env: &Env, plushie_id: u64) -> Result<Plushie, MarketError> {
    env.storage()
        .persistent()
        .get(&DataKey::Plushie(plushie_id))
        .ok_or(MarketError::PlushieNotFound)
}

fn write_plushie(env: &Env, plushie: &Plushie) {
    env.storage()
        .persistent()
        .set(&DataKey::Plushie(plushie.id), plushie);
}

fn read_sale(env: &Env, plushie_id: u64) -> Result<Sale, MarketError> {
    env.storage()
        .persistent()
        .get(&DataKey::Sale(plushie_id))
        .ok_or(MarketError::SaleNotFound)
}

fn read_auction(env: &Env, plushie_id: u64) -> Result<Auction, MarketError> {
    env.storage()
        .persistent()
        .get(&DataKey::Auction(plushie_id))
        .ok_or(MarketError::AuctionNotFound)
}

fn ensure_not_listed(env: &Env, plushie_id: u64) -> Result<(), MarketError> {
    if env.storage().persistent().has(&DataKey::Sale(plushie_id))
        || env
            .storage()
            .persistent()
            .has(&DataKey::Auction(plushie_id))
    {
        Err(MarketError::AlreadyListed)
    } else {
        Ok(())
    }
}

fn require_owner(plushie: &Plushie, owner: &Address) -> Result<(), MarketError> {
    if &plushie.owner == owner {
        Ok(())
    } else {
        Err(MarketError::NotOwner)
    }
}
