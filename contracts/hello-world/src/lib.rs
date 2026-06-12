#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, token, Address, Env, String, Vec,
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Rarity {
    Common,
    Rare,
    Epic,
    Legendary,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ListingType {
    Unlisted,
    FixedPrice,
    Auction,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Plushie {
    pub id: u32,
    pub name: String,
    pub collection: String,
    pub rarity: Rarity,
    pub listing_type: ListingType,
    pub seller: Option<Address>,
    pub asking_price: i128,
    pub current_bid: i128,
    pub highest_bidder: Option<Address>,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Admin,
    PaymentToken,
    NextId,
    Plushie(u32),
    Owner(u32),
    Supply(String, String),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum MarketError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    InvalidAmount = 4,
    PlushieNotFound = 5,
    AlreadyListed = 6,
    NotForSale = 7,
    NotAuction = 8,
    BidTooLow = 9,
    SellerCannotBid = 10,
    AuctionHasBid = 11,
    NoBids = 12,
}

#[contract]
pub struct PlushieMarket;

#[contractimpl]
impl PlushieMarket {
    /// Initializes the market with an admin and a Stellar Asset Contract used
    /// as the payment token.
    pub fn initialize(env: Env, admin: Address, payment_token: Address) -> Result<(), MarketError> {
        let storage = env.storage().persistent();
        if storage.has(&DataKey::Admin) {
            return Err(MarketError::AlreadyInitialized);
        }

        storage.set(&DataKey::Admin, &admin);
        storage.set(&DataKey::PaymentToken, &payment_token);
        storage.set(&DataKey::NextId, &1_u32);
        Ok(())
    }

    /// Mints one unique plushie. Minting the same collection/name again raises
    /// its supply, which lowers the token value of every copy.
    pub fn mint_plushie(
        env: Env,
        owner: Address,
        name: String,
        collection: String,
        rarity: Rarity,
    ) -> Result<u32, MarketError> {
        read_admin(&env)?;
        let storage = env.storage().persistent();
        let id: u32 = storage
            .get(&DataKey::NextId)
            .ok_or(MarketError::NotInitialized)?;
        let supply_key = DataKey::Supply(collection.clone(), name.clone());
        let supply: u32 = storage.get(&supply_key).unwrap_or(0);
        let plushie = Plushie {
            id,
            name,
            collection,
            rarity,
            listing_type: ListingType::Unlisted,
            seller: None,
            asking_price: 0,
            current_bid: 0,
            highest_bidder: None,
        };

        storage.set(&DataKey::Plushie(id), &plushie);
        storage.set(&DataKey::Owner(id), &owner);
        storage.set(&supply_key, &supply.saturating_add(1));
        storage.set(&DataKey::NextId, &id.saturating_add(1));
        Ok(id)
    }

    /// Lists a plushie for immediate purchase with the configured token.
    pub fn list_fixed(
        env: Env,
        owner: Address,
        plushie_id: u32,
        price: i128,
    ) -> Result<(), MarketError> {
        validate_positive(price)?;
        require_owner(&env, plushie_id, &owner)?;

        let mut plushie = read_plushie(&env, plushie_id)?;
        require_unlisted(&plushie)?;
        plushie.listing_type = ListingType::FixedPrice;
        plushie.seller = Some(owner);
        plushie.asking_price = price;
        write_plushie(&env, &plushie);
        Ok(())
    }

    /// Pays the seller and transfers plushie ownership atomically.
    pub fn buy(env: Env, plushie_id: u32, buyer: Address) -> Result<(), MarketError> {
        // Required by the Stellar Asset Contract before it can debit buyer.
        buyer.require_auth();
        let mut plushie = read_plushie(&env, plushie_id)?;
        if plushie.listing_type != ListingType::FixedPrice {
            return Err(MarketError::NotForSale);
        }

        let seller = plushie.seller.clone().ok_or(MarketError::NotForSale)?;
        if buyer == seller {
            return Err(MarketError::Unauthorized);
        }

        payment_client(&env)?.transfer(&buyer, &seller, &plushie.asking_price);
        env.storage()
            .persistent()
            .set(&DataKey::Owner(plushie_id), &buyer);
        clear_listing(&mut plushie);
        write_plushie(&env, &plushie);
        Ok(())
    }

    /// Starts an auction. The first bid must be at least `starting_price`.
    pub fn start_auction(
        env: Env,
        owner: Address,
        plushie_id: u32,
        starting_price: i128,
    ) -> Result<(), MarketError> {
        validate_positive(starting_price)?;
        require_owner(&env, plushie_id, &owner)?;

        let mut plushie = read_plushie(&env, plushie_id)?;
        require_unlisted(&plushie)?;
        plushie.listing_type = ListingType::Auction;
        plushie.seller = Some(owner);
        plushie.asking_price = starting_price;
        plushie.current_bid = 0;
        plushie.highest_bidder = None;
        write_plushie(&env, &plushie);
        Ok(())
    }

    /// Escrows a bid inside this contract and immediately refunds the previous
    /// highest bidder.
    pub fn bid(
        env: Env,
        plushie_id: u32,
        bidder: Address,
        amount: i128,
    ) -> Result<(), MarketError> {
        validate_positive(amount)?;
        // Required by the Stellar Asset Contract before it can debit bidder.
        bidder.require_auth();
        let mut plushie = read_plushie(&env, plushie_id)?;
        if plushie.listing_type != ListingType::Auction {
            return Err(MarketError::NotAuction);
        }
        if plushie.seller.as_ref() == Some(&bidder) {
            return Err(MarketError::SellerCannotBid);
        }

        let minimum = if plushie.highest_bidder.is_some() {
            plushie.current_bid.saturating_add(1)
        } else {
            plushie.asking_price
        };
        if amount < minimum {
            return Err(MarketError::BidTooLow);
        }

        let token = payment_client(&env)?;
        let contract_address = env.current_contract_address();
        token.transfer(&bidder, &contract_address, &amount);
        if let Some(previous_bidder) = plushie.highest_bidder {
            token.transfer(&contract_address, &previous_bidder, &plushie.current_bid);
        }

        plushie.current_bid = amount;
        plushie.highest_bidder = Some(bidder);
        write_plushie(&env, &plushie);
        Ok(())
    }

    /// Seller settles the auction: escrowed tokens go to the seller and the
    /// plushie goes to the highest bidder.
    pub fn settle(env: Env, seller: Address, plushie_id: u32) -> Result<Address, MarketError> {
        require_owner(&env, plushie_id, &seller)?;
        let mut plushie = read_plushie(&env, plushie_id)?;
        if plushie.listing_type != ListingType::Auction {
            return Err(MarketError::NotAuction);
        }

        let winner = plushie.highest_bidder.clone().ok_or(MarketError::NoBids)?;
        payment_client(&env)?.transfer(
            &env.current_contract_address(),
            &seller,
            &plushie.current_bid,
        );
        env.storage()
            .persistent()
            .set(&DataKey::Owner(plushie_id), &winner);
        clear_listing(&mut plushie);
        write_plushie(&env, &plushie);
        Ok(winner)
    }

    /// Cancels a fixed-price listing or an auction that has no bids.
    pub fn cancel(env: Env, owner: Address, plushie_id: u32) -> Result<(), MarketError> {
        require_owner(&env, plushie_id, &owner)?;
        let mut plushie = read_plushie(&env, plushie_id)?;
        if plushie.listing_type == ListingType::Unlisted {
            return Err(MarketError::NotForSale);
        }
        if plushie.highest_bidder.is_some() {
            return Err(MarketError::AuctionHasBid);
        }

        clear_listing(&mut plushie);
        write_plushie(&env, &plushie);
        Ok(())
    }

    pub fn transfer(
        env: Env,
        owner: Address,
        plushie_id: u32,
        new_owner: Address,
    ) -> Result<(), MarketError> {
        require_owner(&env, plushie_id, &owner)?;
        let plushie = read_plushie(&env, plushie_id)?;
        require_unlisted(&plushie)?;
        env.storage()
            .persistent()
            .set(&DataKey::Owner(plushie_id), &new_owner);
        Ok(())
    }

    pub fn get_plushie(env: Env, plushie_id: u32) -> Result<Plushie, MarketError> {
        read_plushie(&env, plushie_id)
    }

    pub fn owner_of(env: Env, plushie_id: u32) -> Result<Address, MarketError> {
        read_owner(&env, plushie_id)
    }

    pub fn all_plushies(env: Env) -> Result<Vec<Plushie>, MarketError> {
        let next_id: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::NextId)
            .ok_or(MarketError::NotInitialized)?;
        let mut plushies = Vec::new(&env);
        for id in 1..next_id {
            plushies.push_back(read_plushie(&env, id)?);
        }
        Ok(plushies)
    }

    pub fn supply_of(env: Env, collection: String, name: String) -> Result<u32, MarketError> {
        read_admin(&env)?;
        Ok(env
            .storage()
            .persistent()
            .get(&DataKey::Supply(collection, name))
            .unwrap_or(0))
    }

    /// Token value = 100 * rarity multiplier / supply.
    ///
    /// Common=1x, Rare=3x, Epic=7x, Legendary=15x.
    pub fn token_value(env: Env, plushie_id: u32) -> Result<i128, MarketError> {
        let plushie = read_plushie(&env, plushie_id)?;
        let supply: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::Supply(
                plushie.collection.clone(),
                plushie.name.clone(),
            ))
            .unwrap_or(1);
        let multiplier: i128 = match plushie.rarity {
            Rarity::Common => 1,
            Rarity::Rare => 3,
            Rarity::Epic => 7,
            Rarity::Legendary => 15,
        };
        Ok(100_i128
            .saturating_mul(multiplier)
            .checked_div(i128::from(supply))
            .unwrap_or(0))
    }
}

fn validate_positive(amount: i128) -> Result<(), MarketError> {
    if amount <= 0 {
        return Err(MarketError::InvalidAmount);
    }
    Ok(())
}

fn require_unlisted(plushie: &Plushie) -> Result<(), MarketError> {
    if plushie.listing_type != ListingType::Unlisted {
        return Err(MarketError::AlreadyListed);
    }
    Ok(())
}

fn require_owner(env: &Env, plushie_id: u32, claimed_owner: &Address) -> Result<(), MarketError> {
    if read_owner(env, plushie_id)? != *claimed_owner {
        return Err(MarketError::Unauthorized);
    }
    Ok(())
}

fn clear_listing(plushie: &mut Plushie) {
    plushie.listing_type = ListingType::Unlisted;
    plushie.seller = None;
    plushie.asking_price = 0;
    plushie.current_bid = 0;
    plushie.highest_bidder = None;
}

fn payment_client(env: &Env) -> Result<token::Client<'_>, MarketError> {
    let token_address: Address = env
        .storage()
        .persistent()
        .get(&DataKey::PaymentToken)
        .ok_or(MarketError::NotInitialized)?;
    Ok(token::Client::new(env, &token_address))
}

fn read_admin(env: &Env) -> Result<Address, MarketError> {
    env.storage()
        .persistent()
        .get(&DataKey::Admin)
        .ok_or(MarketError::NotInitialized)
}

fn read_owner(env: &Env, plushie_id: u32) -> Result<Address, MarketError> {
    env.storage()
        .persistent()
        .get(&DataKey::Owner(plushie_id))
        .ok_or(MarketError::PlushieNotFound)
}

fn read_plushie(env: &Env, plushie_id: u32) -> Result<Plushie, MarketError> {
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

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{
        testutils::Address as _,
        token::{StellarAssetClient, TokenClient},
    };

    struct Fixture {
        env: Env,
        admin: Address,
        buyer_one: Address,
        buyer_two: Address,
        token_id: Address,
        contract_id: Address,
    }

    impl Fixture {
        fn new() -> Self {
            let env = Env::default();
            env.mock_all_auths();

            let admin = Address::generate(&env);
            let buyer_one = Address::generate(&env);
            let buyer_two = Address::generate(&env);
            let token_id = env
                .register_stellar_asset_contract_v2(admin.clone())
                .address();
            let token_admin = StellarAssetClient::new(&env, &token_id);
            token_admin.mint(&buyer_one, &10_000);
            token_admin.mint(&buyer_two, &10_000);

            let contract_id = env.register(PlushieMarket, ());
            PlushieMarketClient::new(&env, &contract_id).initialize(&admin, &token_id);
            Self {
                env,
                admin,
                buyer_one,
                buyer_two,
                token_id,
                contract_id,
            }
        }

        fn market(&self) -> PlushieMarketClient<'_> {
            PlushieMarketClient::new(&self.env, &self.contract_id)
        }

        fn token(&self) -> TokenClient<'_> {
            TokenClient::new(&self.env, &self.token_id)
        }

        fn mint(&self, name: &str, rarity: Rarity) -> u32 {
            self.market().mint_plushie(
                &self.admin,
                &String::from_str(&self.env, name),
                &String::from_str(&self.env, "Cosmic Friends"),
                &rarity,
            )
        }
    }

    #[test]
    fn rare_plushies_have_more_tokens_and_high_supply_has_less() {
        let f = Fixture::new();
        let legendary = f.mint("Moon Bunny", Rarity::Legendary);
        assert_eq!(f.market().token_value(&legendary), 1_500);

        let common = f.mint("Moon Bunny", Rarity::Common);
        assert_eq!(f.market().token_value(&legendary), 750);
        assert_eq!(f.market().token_value(&common), 50);
    }

    #[test]
    fn listing_does_not_require_an_owner_signature() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let owner = Address::from_str(
            &env,
            "GCRF4OIPX5NTWZKNOGYZLFHGDBWQ7ZNVQFM5EQOC6R27OZ6NLH2PVRSY",
        );
        let payment_token = Address::generate(&env);
        let contract_id = env.register(PlushieMarket, ());
        let market = PlushieMarketClient::new(&env, &contract_id);

        market.initialize(&admin, &payment_token);
        let id = market.mint_plushie(
            &owner,
            &String::from_str(&env, "Unsigned Bunny"),
            &String::from_str(&env, "Demo"),
            &Rarity::Common,
        );
        market.list_fixed(&owner, &id, &100);

        assert_eq!(market.get_plushie(&id).asking_price, 100);
    }

    #[test]
    fn fixed_price_purchase_transfers_tokens_and_ownership() {
        let f = Fixture::new();
        let id = f.mint("Star Cat", Rarity::Rare);

        f.market().list_fixed(&f.admin, &id, &250);
        f.market().buy(&id, &f.buyer_one);

        assert_eq!(f.market().owner_of(&id), f.buyer_one);
        assert_eq!(f.token().balance(&f.buyer_one), 9_750);
        assert_eq!(f.token().balance(&f.admin), 250);
    }

    #[test]
    fn auction_escrows_refunds_and_settles() {
        let f = Fixture::new();
        let id = f.mint("Galaxy Fox", Rarity::Epic);

        f.market().start_auction(&f.admin, &id, &100);
        f.market().bid(&id, &f.buyer_one, &150);
        assert_eq!(f.token().balance(&f.contract_id), 150);

        f.market().bid(&id, &f.buyer_two, &225);
        assert_eq!(f.token().balance(&f.buyer_one), 10_000);
        assert_eq!(f.token().balance(&f.contract_id), 225);

        assert_eq!(f.market().settle(&f.admin, &id), f.buyer_two);
        assert_eq!(f.market().owner_of(&id), f.buyer_two);
        assert_eq!(f.token().balance(&f.contract_id), 0);
        assert_eq!(f.token().balance(&f.admin), 225);
    }

    #[test]
    fn rejects_low_bid_and_cancel_after_bid() {
        let f = Fixture::new();
        let id = f.mint("Cloud Bear", Rarity::Common);
        f.market().start_auction(&f.admin, &id, &100);

        assert_eq!(
            f.market().try_bid(&id, &f.buyer_one, &99),
            Err(Ok(MarketError::BidTooLow))
        );
        f.market().bid(&id, &f.buyer_one, &100);
        assert_eq!(
            f.market().try_cancel(&f.admin, &id),
            Err(Ok(MarketError::AuctionHasBid))
        );
    }
}
