use crate::prelude::*;

pub struct OptifiClient {
    pub cluster: Cluster,
    pub program: Program,
    pub optifi_exchange: Pubkey,
    pub user_account: Pubkey,
    pub usdc_token_mint: Pubkey,
    pub token_program: Pubkey,
    pub system_program: Pubkey,
    pub rent: Pubkey,
    pub account: OptifiAccount,
}

unsafe impl Send for OptifiClient {}
unsafe impl Sync for OptifiClient {}

#[derive(Debug)]
pub struct OptifiOrder {
    pub side: OrderSide,
    pub price: f64,
    pub size: f64,
    pub client_order_id: u64,
}

pub struct OptifiAccount {
    pub optifi_exchange: Option<Exchange>,
    pub user_account: Option<UserAccount>,
    pub markets: Vec<Market>,
}

pub struct Market {
    pub optifi_market: OptifiMarket,
    pub optifi_market_key_data: OptifiMarketKeyData,
    pub instrument_common: InstrumentCommon,
    pub strike: u32,
    pub instrument_type: InstrumentType,
    pub market_pubkeys: MarketPubkeys,
    pub serum_account: solana_sdk::account::Account,
}

impl Clone for Market {
    fn clone(&self) -> Self {
        Self {
            optifi_market: self.optifi_market.clone(),
            optifi_market_key_data: self.optifi_market_key_data.clone(),
            instrument_common: self.instrument_common.clone(),
            strike: self.strike.clone(),
            instrument_type: self.instrument_type.clone(),
            market_pubkeys: MarketPubkeys {
                market: self.market_pubkeys.market.clone(),
                req_q: self.market_pubkeys.req_q.clone(),
                event_q: self.market_pubkeys.event_q.clone(),
                bids: self.market_pubkeys.bids.clone(),
                asks: self.market_pubkeys.asks.clone(),
                coin_vault: self.market_pubkeys.coin_vault.clone(),
                pc_vault: self.market_pubkeys.pc_vault.clone(),
                vault_signer_key: self.market_pubkeys.vault_signer_key.clone(),
            },
            serum_account: self.serum_account.clone(),
        }
    }
}

impl std::fmt::Debug for Market {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Market")
            .field("optifi_market", &self.optifi_market)
            .field("optifi_market_key_data", &self.optifi_market_key_data)
            .field("instrument_common", &self.instrument_common)
            .field("strike", &self.strike)
            .field("instrument_type", &self.instrument_type)
            .field("market_pubkeys", &self.market_pubkeys)
            .finish()
    }
}

impl OptifiClient {
    pub fn new(cluster: Cluster, wallet_path: Option<String>, delegator: Option<Pubkey>) -> Self {
        let optifi_exchange = Pubkey::from_str(OPTIFI_EXCHANGE).unwrap();

        // Wallet and cluster params.
        let wallet_path = wallet_path.unwrap_or("~/.config/solana/id.json".to_string());

        let payer = read_keypair_file(&*shellexpand::tilde(&wallet_path))
            .expect("Example requires a keypair file");

        let user = payer.pubkey();

        // Client.
        let client = Client::new_with_options(
            cluster.clone(),
            Rc::new(payer),
            CommitmentConfig::processed(),
        );

        // Program client.
        let program = client.program(optifi_cpi::id());

        let (user_account_key, ..) = if delegator.is_some() {
            (delegator.unwrap(), 0)
        } else {
            get_user_account_pda(&optifi_exchange, &user, &optifi_cpi::id())
        };

        let usdc_token_mint = Pubkey::from_str(USDC_TOKEN_MINT).unwrap();
        let token_program = spl_token::id();
        let system_program = solana_program::system_program::id();
        let rent = solana_program::sysvar::rent::id();

        let account = OptifiAccount {
            optifi_exchange: None,
            user_account: None,
            markets: vec![],
        };

        let optifi_client = Self {
            cluster,
            program,
            optifi_exchange,
            user_account: user_account_key,
            usdc_token_mint,
            token_program,
            system_program,
            rent,
            account,
        };

        optifi_client
    }

    pub fn initialize(
        cluster: Cluster,
        wallet_path: Option<String>,
        delegator: Option<Pubkey>,
    ) -> Self {
        let mut optifi_client = OptifiClient::new(cluster, wallet_path, delegator);

        optifi_client.load_optifi_exchange();
        optifi_client.load_user_account();
        optifi_client.load_markets();

        optifi_client
    }

    pub fn load_optifi_exchange(&mut self) {
        loop {
            match self.program.account(self.optifi_exchange) {
                Ok(result) => {
                    self.account.optifi_exchange = Some(result);
                    return;
                }
                Err(error) => {
                    println!("reload optifi exchange error: {}", error);
                    sleep(time::Duration::from_secs(10))
                }
            }
        }
    }

    pub fn load_user_account(&mut self) {
        loop {
            match self.program.account(self.user_account) {
                Ok(result) => {
                    self.account.user_account = Some(result);
                    return;
                }
                Err(error) => {
                    println!("reload user account {} error: {}", self.user_account, error);
                    sleep(time::Duration::from_secs(10))
                }
            }
        }
    }

    pub fn load_markets(&mut self /*, asset: Option<Asset> */) {
        let optifi_exchange = self.account.optifi_exchange.as_ref().unwrap();

        let optifi_markets = self.program.accounts::<OptifiMarket>(vec![]).unwrap();

        // println!("{:#?}", optifi_exchange);

        // println!("{:?}", optifi_markets);

        let mut markets = vec![];

        for optifi_market_key_data in optifi_exchange.markets.iter() {
            if optifi_market_key_data.is_stopped {
                continue;
            }

            let optifi_market_pubkey = optifi_market_key_data.optifi_market_pubkey;

            // let optifi_market: OptifiMarket = loop {
            //     match self.program.account(optifi_market_pubkey) {
            //         Ok(result) => {
            //             break result;
            //         }
            //         Err(error) => {
            //             println!("{}", error);
            //             sleep(time::Duration::from_secs(10))
            //         }
            //     }
            // };

            let optifi_market: OptifiMarket = optifi_markets
                .iter()
                .find_map(|(pubkey, optifi_market)| {
                    if pubkey == &optifi_market_pubkey {
                        Some(optifi_market.clone())
                    } else {
                        None
                    }
                })
                .unwrap();

            let instrument_pubkey = optifi_market.instrument;

            // println!("{}", instrument_pubkey);

            let (instrument_common, strike, is_call) = optifi_exchange
                .get_instrument_data(&instrument_pubkey)
                .unwrap();

            // if let Some(asset) = asset {
            //     if asset != instrument_common.asset {
            //         continue;
            //     }
            // }

            let instrument_type = if is_call {
                InstrumentType::Call
            } else {
                InstrumentType::Put
            };

            let serum_dex_program_id = Pubkey::from_str(SERUM_DEX_PROGRAM_ID).unwrap();

            let market_pubkeys: MarketPubkeys = get_keys_for_market(
                &self.program.rpc(),
                &serum_dex_program_id,
                &optifi_market.serum_market,
            )
            .unwrap();

            let serum_account: solana_sdk::account::Account = self
                .program
                .rpc()
                .get_account_with_commitment(
                    &optifi_market.serum_market,
                    CommitmentConfig::processed(),
                )
                .unwrap()
                .value
                .ok_or(ClientError::AccountNotFound)
                .unwrap();

            let market = Market {
                optifi_market,
                optifi_market_key_data: optifi_market_key_data.clone(),
                instrument_common,
                strike,
                instrument_type,
                market_pubkeys,
                serum_account,
            };

            markets.push(market);
        }
        self.account.markets = markets;
    }

    pub fn get_markets(&self, asset: Option<Asset>) -> Vec<Market> {
        self.account
            .markets
            .iter()
            .filter_map(|market| {
                if let Some(asset) = asset {
                    if market.instrument_common.asset == asset {
                        Some(market.to_owned())
                    } else {
                        None
                    }
                } else {
                    Some(market.to_owned())
                }
            })
            .collect()
    }

    pub fn get_user_account(&self) -> &UserAccount {
        self.account.user_account.as_ref().unwrap()
    }

    pub fn get_usdc_balance(&self) -> Result<u64> {
        let pubkey = &self.get_user_account().user_margin_account_usdc;

        let mut account: solana_sdk::account::Account = self
            .program
            .rpc()
            .get_account_with_commitment(pubkey, CommitmentConfig::processed())
            .unwrap()
            .value
            .ok_or(ClientError::AccountNotFound)
            .unwrap();

        let account_info = AccountInfo::new(
            &pubkey,
            false,
            true,
            &mut account.lamports,
            &mut account.data,
            &mut account.owner,
            account.executable,
            account.rent_epoch,
        );

        accessor::amount(&account_info)
    }
    // pub fn get_user_position(&self) -> Vec<(Market, UserPosition)> {
    //     self.account
    //         .user_account
    //         .as_ref()
    //         .unwrap()
    //         .positions
    //         .iter()
    //         .filter_map(|position| {

    //         })
    //         .collect()
    // }

    pub fn initialize_user_account(&self) -> std::result::Result<Signature, ClientError> {
        let user = self.program.payer();

        let (user_account_key, user_account_bump) =
            get_user_account_pda(&self.optifi_exchange, &user, &optifi_cpi::id());

        let user_margin_account_usdc = Keypair::new();

        let (liquidation_account, liquidation_account_bump) = get_user_liquidation_account_pda(
            &self.optifi_exchange,
            &user_account_key,
            &optifi_cpi::id(),
        );

        println!(
            "user_margin_account_usdc: {}",
            user_margin_account_usdc.pubkey()
        );

        // Build and send a transaction.
        let tx = self
            .program
            .request()
            .instruction(system_instruction::create_account(
                &user,
                &user_margin_account_usdc.pubkey(),
                self.program
                    .rpc()
                    .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN)
                    .unwrap(),
                spl_token::state::Account::LEN as u64,
                &self.token_program,
            ))
            .instruction(
                spl_token::instruction::initialize_account(
                    &self.token_program,
                    &user_margin_account_usdc.pubkey(),
                    &self.usdc_token_mint,
                    &user,
                )
                .unwrap(),
            )
            .signer(&user_margin_account_usdc)
            .accounts(optifi_cpi::accounts::InitializeUserAccount {
                optifi_exchange: self.optifi_exchange,
                user_account: user_account_key,
                user_margin_account_usdc: user_margin_account_usdc.pubkey(),
                owner: user,
                payer: user,
                token_program: self.token_program,
                system_program: self.system_program,
                rent: self.rent,
                liquidation_account,
            })
            .args(optifi_cpi::instruction::InitUserAccount {
                bump: InitUserAccountBumpSeeds {
                    user_account: user_account_bump,
                    liquidation_account: liquidation_account_bump,
                },
            })
            .send();

        tx
    }

    pub fn set_delegation(
        &self,
        delegatee: Option<Pubkey>,
    ) -> std::result::Result<Signature, ClientError> {
        let user = self.program.payer();

        let user_account_key = self.user_account;

        println!("set delegation to: {:?}", delegatee);

        // Build and send a transaction.
        let tx = self
            .program
            .request()
            .accounts(optifi_cpi::accounts::SetDelegation {
                optifi_exchange: self.optifi_exchange,
                user_account: user_account_key,
                user,
            })
            .args(optifi_cpi::instruction::SetDelegation { delegatee })
            .send();

        tx
    }

    pub fn deposit(&self, ui_amount: f64) -> std::result::Result<Signature, ClientError> {
        let user = self.program.payer();

        let user_account_key = self.user_account;

        let user_account: UserAccount = self.program.account(user_account_key).unwrap();

        let user_margin_account_usdc = user_account.user_margin_account_usdc;

        let deposit_source = get_associated_token_address(&user, &self.usdc_token_mint);

        let amount = (ui_amount * 1000000.) as u64;

        println!("deposit ui_amount: {}, amount: {}", ui_amount, amount);

        // Build and send a transaction.
        let tx = self
            .program
            .request()
            .accounts(optifi_cpi::accounts::Deposit {
                optifi_exchange: self.optifi_exchange,
                user_account: user_account_key,
                user_margin_account_usdc,
                user,
                deposit_source,
                token_program: self.token_program,
            })
            .args(optifi_cpi::instruction::Deposit { amount })
            .send();

        tx
    }

    pub fn withdraw(&self, ui_amount: f64) -> std::result::Result<Signature, ClientError> {
        let user = self.program.payer();

        let user_account_key = self.user_account;

        let user_account: UserAccount = self.program.account(user_account_key).unwrap();

        let user_margin_account_usdc = user_account.user_margin_account_usdc;

        let withdraw_dest = get_associated_token_address(&user, &self.usdc_token_mint);

        let amount = (ui_amount * 1000000.) as u64;

        println!("withdraw ui_amount: {}, amount: {}", ui_amount, amount);

        // Build and send a transaction.
        let tx = self
            .program
            .request()
            .accounts(optifi_cpi::accounts::Withdraw {
                optifi_exchange: self.optifi_exchange,
                user_account: user_account_key,
                user_margin_account_usdc,
                user,
                withdraw_dest,
                token_program: self.token_program,
            })
            .args(optifi_cpi::instruction::Withdraw { amount })
            .send();

        tx
    }

    pub fn initialize_user_on_market(
        &self,
        market: &Market,
    ) -> std::result::Result<Signature, ClientError> {
        let user = self.program.payer();
        let user_account = self.user_account;

        let serum_dex_program_id = Pubkey::from_str(SERUM_DEX_PROGRAM_ID).unwrap();

        let (serum_market_authority, ..) =
            get_serum_market_auth_pda(&self.optifi_exchange, &optifi_cpi::id());

        let optifi_market = market.optifi_market_key_data.optifi_market_pubkey;
        let serum_market = market.optifi_market.serum_market;

        let (serum_open_orders, bump) = get_serum_open_orders_account(
            &self.optifi_exchange,
            &user_account,
            &serum_market,
            &optifi_cpi::id(),
        );

        // println!("serum_open_orders: {}", serum_open_orders);

        // Build and send a transaction.
        let tx = self
            .program
            .request()
            .instruction(create_associated_token_account(
                &user,
                &user_account,
                &market.optifi_market.instrument_long_spl_token,
            ))
            .instruction(create_associated_token_account(
                &user,
                &user_account,
                &market.optifi_market.instrument_short_spl_token,
            ))
            .accounts(optifi_cpi::accounts::InitUserOnOptifiMarket {
                optifi_exchange: self.optifi_exchange,
                user,
                user_account,

                optifi_market,
                serum_market,
                serum_open_orders,

                serum_dex_program_id,
                serum_market_authority,

                payer: user,
                system_program: self.system_program,
                rent: self.rent,
            })
            .args(optifi_cpi::instruction::InitUserOnOptifiMarket { bump })
            .send();

        tx
    }

    fn get_margin_stress_calculate_instruction(&self, asset: Asset) -> Instruction {
        let exchange = self.account.optifi_exchange.as_ref().unwrap();

        let oracle = exchange.get_oracle(asset);

        let asset_feed = oracle.spot_oracle.unwrap();

        let iv_feed = oracle.iv_oracle.unwrap();

        let usdc_feed = exchange.get_oracle(Asset::USDC).spot_oracle.unwrap();

        let (margin_stress_account, ..) =
            get_margin_stress_account(&self.optifi_exchange, asset as u8, &optifi_cpi::id());

        let ix = self
            .program
            .request()
            .accounts(optifi_cpi::accounts::CalculateMarginStressContext {
                optifi_exchange: self.optifi_exchange,
                margin_stress_account,
                asset_feed,
                iv_feed,
                usdc_feed,
            })
            .args(optifi_cpi::instruction::MarginStressCalculate {})
            .instructions()
            .unwrap()
            .pop()
            .unwrap();

        ix
    }

    // pub fn load_all_open_orders(&self) -> Vec<(&Market, OptifiOrder)> {}

    pub fn load_open_orders(&self, market: &Market) -> Vec<OptifiOrder> {
        let serum_market = market.optifi_market.serum_market;

        let asset = market.instrument_common.asset;

        let serum_dex_program_id = Pubkey::from_str(SERUM_DEX_PROGRAM_ID).unwrap();

        let (open_orders, ..) = get_serum_open_orders_account(
            &self.optifi_exchange,
            &self.user_account,
            &serum_market,
            &optifi_cpi::id(),
        );

        let mut market_account = market.serum_account.clone();

        let market_account_info = AccountInfo::new(
            &serum_market,
            false,
            false,
            &mut market_account.lamports,
            &mut market_account.data,
            &mut market_account.owner,
            market_account.executable,
            market_account.rent_epoch,
        );

        let mut orders_account = match self
            .program
            .rpc()
            .get_account_with_commitment(&open_orders, CommitmentConfig::processed())
            .unwrap()
            .value
            .ok_or(ClientError::AccountNotFound)
        {
            Ok(account) => account,
            Err(_) => {
                println!("AccountNotFound");
                return vec![];
            }
        };

        let orders_account_info = AccountInfo::new(
            &open_orders,
            false,
            false,
            &mut orders_account.lamports,
            &mut orders_account.data,
            &mut orders_account.owner,
            orders_account.executable,
            orders_account.rent_epoch,
        );

        let serum_market_pubkeys: &MarketPubkeys = &market.market_pubkeys;

        let mut asks_account = self
            .program
            .rpc()
            .get_account_with_commitment(&serum_market_pubkeys.asks, CommitmentConfig::processed())
            .unwrap()
            .value
            .ok_or(ClientError::AccountNotFound)
            .unwrap();

        let asks_account_info = AccountInfo::new(
            &serum_market_pubkeys.asks,
            false,
            true,
            &mut asks_account.lamports,
            &mut asks_account.data,
            &mut asks_account.owner,
            asks_account.executable,
            asks_account.rent_epoch,
        );

        let mut bids_account = self
            .program
            .rpc()
            .get_account_with_commitment(&serum_market_pubkeys.bids, CommitmentConfig::processed())
            .unwrap()
            .value
            .ok_or(ClientError::AccountNotFound)
            .unwrap();

        let bids_account_info = AccountInfo::new(
            &serum_market_pubkeys.bids,
            false,
            true,
            &mut bids_account.lamports,
            &mut bids_account.data,
            &mut bids_account.owner,
            bids_account.executable,
            bids_account.rent_epoch,
        );

        let market =
            serum_dex::state::Market::load(&market_account_info, &serum_dex_program_id, false)
                .unwrap();

        let open_orders = market
            .load_orders_mut(
                &orders_account_info,
                None,
                &serum_dex_program_id,
                None,
                None,
            )
            .unwrap();

        let mut asks = market
            .load_asks_mut(&asks_account_info)
            .map_err(|err| Error::ProgramError(ProgramError::from(err).into()))
            .unwrap();

        let mut bids = market
            .load_bids_mut(&bids_account_info)
            .map_err(|err| Error::ProgramError(ProgramError::from(err).into()))
            .unwrap();

        // let client_order_ids = open_orders.client_order_ids;

        // println!("{:#?}", client_order_ids);

        let mut orders: Vec<OptifiOrder> = vec![];

        for order_id in open_orders.orders.into_iter() {
            if let Some(key) = asks.find_by_key(order_id) {
                let node = asks.deref_mut().get(key).unwrap().as_leaf().unwrap();
                let order = OptifiOrder {
                    side: OrderSide::Ask,
                    price: u64::from(node.price()) as f64
                        / 10_u32.pow(USDC_DECIMALS - asset.get_decimal()) as f64,
                    size: node.quantity() as f64 / 10_u32.pow(asset.get_decimal()) as f64,
                    client_order_id: node.client_order_id(),
                };
                orders.push(order);
            } else if let Some(key) = bids.find_by_key(order_id) {
                let node = bids.deref_mut().get(key).unwrap().as_leaf().unwrap();
                let order = OptifiOrder {
                    side: OrderSide::Bid,
                    price: u64::from(node.price()) as f64
                        / 10_u32.pow(USDC_DECIMALS - asset.get_decimal()) as f64,
                    size: node.quantity() as f64 / 10_u32.pow(asset.get_decimal()) as f64,
                    client_order_id: node.client_order_id(),
                };
                orders.push(order);
            }
        }
        orders
    }

    pub fn load_order_book(&self, market: &Market) -> Book {
        let serum_market = market.optifi_market.serum_market;

        let asset = market.instrument_common.asset;

        let serum_dex_program_id = Pubkey::from_str(SERUM_DEX_PROGRAM_ID).unwrap();

        let mut market_account = market.serum_account.clone();

        let market_account_info = AccountInfo::new(
            &serum_market,
            false,
            false,
            &mut market_account.lamports,
            &mut market_account.data,
            &mut market_account.owner,
            market_account.executable,
            market_account.rent_epoch,
        );

        let serum_market_pubkeys: &MarketPubkeys = &market.market_pubkeys;

        let mut asks_account = self
            .program
            .rpc()
            .get_account_with_commitment(&serum_market_pubkeys.asks, CommitmentConfig::processed())
            .unwrap()
            .value
            .ok_or(ClientError::AccountNotFound)
            .unwrap();

        let asks_account_info = AccountInfo::new(
            &serum_market_pubkeys.asks,
            false,
            true,
            &mut asks_account.lamports,
            &mut asks_account.data,
            &mut asks_account.owner,
            asks_account.executable,
            asks_account.rent_epoch,
        );

        let mut bids_account = self
            .program
            .rpc()
            .get_account_with_commitment(&serum_market_pubkeys.bids, CommitmentConfig::processed())
            .unwrap()
            .value
            .ok_or(ClientError::AccountNotFound)
            .unwrap();

        let bids_account_info = AccountInfo::new(
            &serum_market_pubkeys.bids,
            false,
            true,
            &mut bids_account.lamports,
            &mut bids_account.data,
            &mut bids_account.owner,
            bids_account.executable,
            bids_account.rent_epoch,
        );

        let market =
            serum_dex::state::Market::load(&market_account_info, &serum_dex_program_id, false)
                .unwrap();

        let asks = market
            .load_asks_mut(&asks_account_info)
            .map_err(|err| Error::ProgramError(ProgramError::from(err).into()))
            .unwrap();

        let bids = market
            .load_bids_mut(&bids_account_info)
            .map_err(|err| Error::ProgramError(ProgramError::from(err).into()))
            .unwrap();

        let mut ask_levels: Vec<BookLevel> = vec![];

        for node in asks.traverse().iter() {
            let order = OptifiOrder {
                side: OrderSide::Ask,
                price: u64::from(node.price()) as f64
                    / 10_u32.pow(USDC_DECIMALS - asset.get_decimal()) as f64,
                size: node.quantity() as f64 / 10_u32.pow(asset.get_decimal()) as f64,
                client_order_id: node.client_order_id(),
            };
            // println!("{:#?}", order);

            if let Some(level) = ask_levels
                .iter_mut()
                .find(|level| level.price == order.price)
            {
                level.size += order.size;
            } else {
                ask_levels.push(BookLevel {
                    price: order.price,
                    size: order.size,
                });
            }
        }

        let mut bid_levels: Vec<BookLevel> = vec![];

        for node in bids.traverse().iter() {
            let order = OptifiOrder {
                side: OrderSide::Bid,
                price: u64::from(node.price()) as f64
                    / 10_u32.pow(USDC_DECIMALS - asset.get_decimal()) as f64,
                size: node.quantity() as f64 / 10_u32.pow(asset.get_decimal()) as f64,
                client_order_id: node.client_order_id(),
            };
            // println!("{:#?}", order);

            if let Some(level) = bid_levels
                .iter_mut()
                .find(|level| level.price == order.price)
            {
                level.size += order.size;
            } else {
                bid_levels.push(BookLevel {
                    price: order.price,
                    size: order.size,
                });
            }
        }

        Book {
            bids: bid_levels,
            asks: ask_levels,
        }
    }

    pub fn place_order(
        &self,
        market: &Market,
        side: OrderSide,
        price: f64,
        size: f64,
        order_type: OrderType,
    ) -> std::result::Result<Signature, ClientError> {
        let user = self.program.payer();
        let user_account = self.user_account;

        let serum_dex_program_id = Pubkey::from_str(SERUM_DEX_PROGRAM_ID).unwrap();

        let optifi_market = market.optifi_market_key_data.optifi_market_pubkey;
        let serum_market = market.optifi_market.serum_market;

        let (open_orders, ..) = get_serum_open_orders_account(
            &self.optifi_exchange,
            &user_account,
            &serum_market,
            &optifi_cpi::id(),
        );

        let serum_market_pubkeys: &MarketPubkeys = &market.market_pubkeys;

        // println!("{:#?}", market);

        let asset = market.instrument_common.asset;

        let (margin_stress_account, ..) =
            get_margin_stress_account(&self.optifi_exchange, asset as u8, &optifi_cpi::id());

        let usdc_fee_pool = self.account.optifi_exchange.as_ref().unwrap().usdc_fee_pool;

        let user_margin_account = self
            .account
            .user_account
            .as_ref()
            .unwrap()
            .user_margin_account_usdc;

        let (instrument_token_mint_authority_pda, ..) =
            get_optifi_market_mint_auth_pda(&self.optifi_exchange, &optifi_cpi::id());

        let user_instrument_long_token_vault = get_associated_token_address(
            &user_account,
            &market.optifi_market.instrument_long_spl_token,
        );

        let user_instrument_short_token_vault = get_associated_token_address(
            &user_account,
            &market.optifi_market.instrument_short_spl_token,
        );

        // Calculation

        let limit = (price * 10_u32.pow(USDC_DECIMALS - asset.get_decimal()) as f64) as u64;

        let max_coin_qty = (size * 10_u32.pow(asset.get_decimal()) as f64) as u64;

        let max_pc_qty = limit * max_coin_qty;

        let max_pc_qty = (max_pc_qty as f64 * (1.0 + TAKER_FEE)) as u64;

        // Margin Stress

        let ix_2 = self.get_margin_stress_calculate_instruction(asset);

        let ix_3 = self
            .program
            .request()
            .accounts(optifi_cpi::accounts::PlaceOrderContext {
                optifi_exchange: self.optifi_exchange,

                user,
                user_account,
                user_margin_account,

                optifi_market,
                serum_market,
                open_orders,

                asks: *serum_market_pubkeys.asks,
                bids: *serum_market_pubkeys.bids,
                pc_vault: *serum_market_pubkeys.pc_vault,
                coin_vault: *serum_market_pubkeys.coin_vault,
                request_queue: *serum_market_pubkeys.req_q,
                event_queue: *serum_market_pubkeys.event_q,
                vault_signer: *serum_market_pubkeys.vault_signer_key,

                coin_mint: market.optifi_market.instrument_long_spl_token,
                instrument_short_spl_token_mint: market.optifi_market.instrument_short_spl_token,
                instrument_token_mint_authority_pda,
                user_instrument_long_token_vault,
                user_instrument_short_token_vault,

                usdc_fee_pool,

                margin_stress_account,

                serum_dex_program_id,
                token_program: self.token_program,
                rent: self.rent,
            })
            .args(optifi_cpi::instruction::PlaceOrder {
                side,
                limit: limit as u64,
                max_coin_qty: max_coin_qty as u64,
                max_pc_qty,
                order_type: order_type as u8,
            })
            .instructions()
            .unwrap()
            .pop()
            .unwrap();

        let ix_4 = serum_dex::instruction::consume_events(
            &serum_dex_program_id,
            vec![&open_orders],
            &serum_market,
            &serum_market_pubkeys.event_q,
            &user_instrument_long_token_vault,
            &user_margin_account,
            65535,
        )
        .unwrap();

        let ix_5 = self
            .program
            .request()
            .accounts(optifi_cpi::accounts::OrderSettlement {
                optifi_exchange: self.optifi_exchange,

                user_account,
                user_margin_account,

                optifi_market,
                serum_market,
                user_serum_open_orders: open_orders,

                asks: *serum_market_pubkeys.asks,
                bids: *serum_market_pubkeys.bids,
                pc_vault: *serum_market_pubkeys.pc_vault,
                coin_vault: *serum_market_pubkeys.coin_vault,
                request_queue: *serum_market_pubkeys.req_q,
                event_queue: *serum_market_pubkeys.event_q,
                vault_signer: *serum_market_pubkeys.vault_signer_key,

                instrument_long_spl_token_mint: market.optifi_market.instrument_long_spl_token,
                instrument_short_spl_token_mint: market.optifi_market.instrument_short_spl_token,
                user_instrument_long_token_vault,
                user_instrument_short_token_vault,

                serum_dex_program_id,
                token_program: self.token_program,
            })
            .args(optifi_cpi::instruction::SettleOrderFunds {})
            .instructions()
            .unwrap()
            .pop()
            .unwrap();

        let ix_6 = self
            .program
            .request()
            .accounts(optifi_cpi::accounts::MarginContext {
                optifi_exchange: self.optifi_exchange,
                user_account,
                margin_stress_account,
                clock: solana_program::sysvar::clock::id(),
            })
            .args(optifi_cpi::instruction::UserMarginCalculate {})
            .instructions()
            .unwrap()
            .pop()
            .unwrap();

        // Build and send a transaction.
        let tx = self
            .program
            .request()
            .instruction(ComputeBudgetInstruction::request_units(1400000, 0))
            .instruction(ix_2)
            .instruction(ix_3)
            .instruction(ix_4)
            .instruction(ix_5)
            .instruction(ix_6)
            .send();

        tx
    }

    pub fn settle_order(&self, market: &Market) -> std::result::Result<Signature, ClientError> {
        let user_account = self.user_account;

        let serum_dex_program_id = Pubkey::from_str(SERUM_DEX_PROGRAM_ID).unwrap();

        let optifi_market = market.optifi_market_key_data.optifi_market_pubkey;
        let serum_market = market.optifi_market.serum_market;

        let (open_orders, ..) = get_serum_open_orders_account(
            &self.optifi_exchange,
            &user_account,
            &serum_market,
            &optifi_cpi::id(),
        );

        let serum_market_pubkeys: &MarketPubkeys = &market.market_pubkeys;

        let user_margin_account = self
            .account
            .user_account
            .as_ref()
            .unwrap()
            .user_margin_account_usdc;

        let user_instrument_long_token_vault = get_associated_token_address(
            &user_account,
            &market.optifi_market.instrument_long_spl_token,
        );

        let user_instrument_short_token_vault = get_associated_token_address(
            &user_account,
            &market.optifi_market.instrument_short_spl_token,
        );

        let asset = market.instrument_common.asset;

        let ix_2 = self.get_margin_stress_calculate_instruction(asset);

        let ix_4 = serum_dex::instruction::consume_events(
            &serum_dex_program_id,
            vec![&open_orders],
            &serum_market,
            &serum_market_pubkeys.event_q,
            &user_instrument_long_token_vault,
            &user_margin_account,
            65535,
        )
        .unwrap();

        let ix_5 = self
            .program
            .request()
            .accounts(optifi_cpi::accounts::OrderSettlement {
                optifi_exchange: self.optifi_exchange,

                user_account,
                user_margin_account,

                optifi_market,
                serum_market,
                user_serum_open_orders: open_orders,

                asks: *serum_market_pubkeys.asks,
                bids: *serum_market_pubkeys.bids,
                pc_vault: *serum_market_pubkeys.pc_vault,
                coin_vault: *serum_market_pubkeys.coin_vault,
                request_queue: *serum_market_pubkeys.req_q,
                event_queue: *serum_market_pubkeys.event_q,
                vault_signer: *serum_market_pubkeys.vault_signer_key,

                instrument_long_spl_token_mint: market.optifi_market.instrument_long_spl_token,
                instrument_short_spl_token_mint: market.optifi_market.instrument_short_spl_token,
                user_instrument_long_token_vault,
                user_instrument_short_token_vault,

                serum_dex_program_id,
                token_program: self.token_program,
            })
            .args(optifi_cpi::instruction::SettleOrderFunds {})
            .instructions()
            .unwrap()
            .pop()
            .unwrap();

        let asset = market.instrument_common.asset;

        let (margin_stress_account, ..) =
            get_margin_stress_account(&self.optifi_exchange, asset as u8, &optifi_cpi::id());

        let ix_6 = self
            .program
            .request()
            .accounts(optifi_cpi::accounts::MarginContext {
                optifi_exchange: self.optifi_exchange,
                user_account,
                margin_stress_account,
                clock: solana_program::sysvar::clock::id(),
            })
            .args(optifi_cpi::instruction::UserMarginCalculate {})
            .instructions()
            .unwrap()
            .pop()
            .unwrap();

        // Build and send a transaction.
        let tx = self
            .program
            .request()
            .instruction(ix_2)
            .instruction(ix_4)
            .instruction(ix_5)
            .instruction(ix_6)
            .send();

        tx
    }

    pub fn cancel_order(
        &self,
        market: &Market,
        side: OrderSide,
        client_order_id: u64,
    ) -> std::result::Result<Signature, ClientError> {
        let user = self.program.payer();
        let user_account = self.user_account;

        let serum_dex_program_id = Pubkey::from_str(SERUM_DEX_PROGRAM_ID).unwrap();

        let optifi_market = market.optifi_market_key_data.optifi_market_pubkey;
        let serum_market = market.optifi_market.serum_market;

        let (open_orders, ..) = get_serum_open_orders_account(
            &self.optifi_exchange,
            &user_account,
            &serum_market,
            &optifi_cpi::id(),
        );

        let serum_market_pubkeys: &MarketPubkeys = &market.market_pubkeys;

        let usdc_fee_pool = self.account.optifi_exchange.as_ref().unwrap().usdc_fee_pool;

        let user_margin_account = self
            .account
            .user_account
            .as_ref()
            .unwrap()
            .user_margin_account_usdc;

        let user_instrument_long_token_vault = get_associated_token_address(
            &user_account,
            &market.optifi_market.instrument_long_spl_token,
        );

        let user_instrument_short_token_vault = get_associated_token_address(
            &user_account,
            &market.optifi_market.instrument_short_spl_token,
        );

        let (central_usdc_pool_auth, ..) =
            get_central_usdc_pool_auth_pda(&self.optifi_exchange, &optifi_cpi::id());

        let asset = market.instrument_common.asset;

        let ix_2 = self.get_margin_stress_calculate_instruction(asset);

        let ix_3 = self
            .program
            .request()
            .accounts(optifi_cpi::accounts::CancelOrderContext {
                optifi_exchange: self.optifi_exchange,

                user,
                user_account,
                user_margin_account,

                serum_market,
                open_orders,

                asks: *serum_market_pubkeys.asks,
                bids: *serum_market_pubkeys.bids,
                event_queue: *serum_market_pubkeys.event_q,

                usdc_fee_pool,
                central_usdc_pool_auth,

                serum_dex_program_id,
                token_program: self.token_program,
            })
            .args(optifi_cpi::instruction::CancelOrderByClientOrderId {
                side,
                client_order_id,
            })
            .instructions()
            .unwrap()
            .pop()
            .unwrap();

        let ix_4 = serum_dex::instruction::consume_events(
            &serum_dex_program_id,
            vec![&open_orders],
            &serum_market,
            &serum_market_pubkeys.event_q,
            &user_instrument_long_token_vault,
            &user_margin_account,
            65535,
        )
        .unwrap();

        let ix_5 = self
            .program
            .request()
            .accounts(optifi_cpi::accounts::OrderSettlement {
                optifi_exchange: self.optifi_exchange,

                user_account,
                user_margin_account,

                optifi_market,
                serum_market,
                user_serum_open_orders: open_orders,

                asks: *serum_market_pubkeys.asks,
                bids: *serum_market_pubkeys.bids,
                pc_vault: *serum_market_pubkeys.pc_vault,
                coin_vault: *serum_market_pubkeys.coin_vault,
                request_queue: *serum_market_pubkeys.req_q,
                event_queue: *serum_market_pubkeys.event_q,
                vault_signer: *serum_market_pubkeys.vault_signer_key,

                instrument_long_spl_token_mint: market.optifi_market.instrument_long_spl_token,
                instrument_short_spl_token_mint: market.optifi_market.instrument_short_spl_token,
                user_instrument_long_token_vault,
                user_instrument_short_token_vault,

                serum_dex_program_id,
                token_program: self.token_program,
            })
            .args(optifi_cpi::instruction::SettleOrderFunds {})
            .instructions()
            .unwrap()
            .pop()
            .unwrap();

        let (margin_stress_account, ..) =
            get_margin_stress_account(&self.optifi_exchange, asset as u8, &optifi_cpi::id());

        let ix_6 = self
            .program
            .request()
            .accounts(optifi_cpi::accounts::MarginContext {
                optifi_exchange: self.optifi_exchange,
                user_account,
                margin_stress_account,
                clock: solana_program::sysvar::clock::id(),
            })
            .args(optifi_cpi::instruction::UserMarginCalculate {})
            .instructions()
            .unwrap()
            .pop()
            .unwrap();

        // Build and send a transaction.
        let tx = self
            .program
            .request()
            .instruction(ComputeBudgetInstruction::request_units(1400000, 0))
            .instruction(ix_2)
            .instruction(ix_3)
            .instruction(ix_4)
            .instruction(ix_5)
            .instruction(ix_6)
            .send();

        tx
    }

    pub fn cancel_all_order(&self, market: &Market) {
        let orders = self.load_open_orders(market);

        for order in orders.iter() {
            let signature = self
                .cancel_order(market, order.side, order.client_order_id)
                .unwrap();

            println!("signature: {:#?}", signature);
        }
    }

    pub fn subscribe_ask(&self, market: &Market) {
        loop {
            let start = Instant::now();

            let (_subscription, receiver) = PubsubClient::account_subscribe(
                self.cluster.ws_url(),
                &market.market_pubkeys.asks,
                Some(RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    data_slice: None,
                    commitment: None,
                }),
            )
            .unwrap();

            loop {
                match receiver.recv() {
                    Ok(ui_account) => {
                        let _levels = parse_asks_inner(market, ui_account);
                        println!("{:#?}", _levels);
                    }
                    Err(_e) => {
                        // println!("{}", _e);
                        println!("Time for one subscription: {:?}", start.elapsed());
                        break;
                    }
                }
            }
        }
    }

    pub fn subscribe_open_orders(&self, market: &Market) {
        let (open_orders, ..) = get_serum_open_orders_account(
            &self.optifi_exchange,
            &self.user_account,
            &market.optifi_market.serum_market,
            &optifi_cpi::id(),
        );

        loop {
            // let start = Instant::now();

            let (_subscription, orders_receiver) = PubsubClient::account_subscribe(
                self.cluster.ws_url(),
                &open_orders,
                Some(RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    data_slice: None,
                    commitment: None,
                }),
            )
            .unwrap();

            loop {
                match orders_receiver.recv() {
                    Ok(orders_ui_account) => {
                        parse_open_orders(market, orders_ui_account);
                    }
                    Err(_e) => {
                        // println!("{}", _e);
                        // println!("Time for one subscription: {:?}", start.elapsed());
                        break;
                    }
                }
            }
        }
    }

    pub fn subscribe_user_account(&self) {
        loop {
            let (_subscription, receiver) = PubsubClient::account_subscribe(
                self.cluster.ws_url(),
                &self.user_account,
                Some(RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    data_slice: None,
                    commitment: None,
                }),
            )
            .unwrap();

            loop {
                match receiver.recv() {
                    Ok(ui_account) => {
                        let user_account = parse_user_account_inner(ui_account);

                        println!("{:#?}", user_account.unwrap());
                    }
                    Err(_e) => {
                        println!("{}", _e);
                        // println!("Time for one subscription: {:?}", start.elapsed());
                        break;
                    }
                }
            }
        }
    }
}

pub fn get_optifi_id() -> Pubkey {
    optifi_cpi::id()
}

pub fn parse_asks(market: &Market, result: &Value) -> Vec<BookLevel> {
    parse_asks_inner(&market, serde_json::from_value(result.clone()).unwrap())
}

pub fn parse_asks_inner(market: &Market, ui_account: Response<UiAccount>) -> Vec<BookLevel> {
    let serum_market = market.optifi_market.serum_market;

    let asset = market.instrument_common.asset;

    let mut market_account = market.serum_account.clone();

    let market_account_info = AccountInfo::new(
        &serum_market,
        false,
        false,
        &mut market_account.lamports,
        &mut market_account.data,
        &mut market_account.owner,
        market_account.executable,
        market_account.rent_epoch,
    );

    let serum_dex_program_id = Pubkey::from_str(SERUM_DEX_PROGRAM_ID).unwrap();

    let serum_market =
        serum_dex::state::Market::load(&market_account_info, &serum_dex_program_id, false).unwrap();

    let mut asks_account = ui_account
        .value
        .decode::<anchor_client::solana_sdk::account::Account>()
        .unwrap();

    let asks_account_info = AccountInfo::new(
        &market.market_pubkeys.asks,
        false,
        true,
        &mut asks_account.lamports,
        &mut asks_account.data,
        &mut asks_account.owner,
        asks_account.executable,
        asks_account.rent_epoch,
    );

    let asks = serum_market
        .load_asks_mut(&asks_account_info)
        .map_err(|err| Error::ProgramError(ProgramError::from(err).into()))
        .unwrap();

    let mut ask_levels: Vec<BookLevel> = vec![];

    for node in asks.traverse().iter() {
        let order = OptifiOrder {
            side: OrderSide::Ask,
            price: u64::from(node.price()) as f64
                / 10_u32.pow(USDC_DECIMALS - asset.get_decimal()) as f64,
            size: node.quantity() as f64 / 10_u32.pow(asset.get_decimal()) as f64,
            client_order_id: node.client_order_id(),
        };
        // println!("{:#?}", order);

        if let Some(level) = ask_levels
            .iter_mut()
            .find(|level| level.price == order.price)
        {
            level.size += order.size;
        } else {
            ask_levels.push(BookLevel {
                price: order.price,
                size: order.size,
            });
        }
    }

    ask_levels
}

pub fn parse_bids(market: &Market, result: &Value) -> Vec<BookLevel> {
    parse_bids_inner(&market, serde_json::from_value(result.clone()).unwrap())
}

pub fn parse_bids_inner(market: &Market, ui_account: Response<UiAccount>) -> Vec<BookLevel> {
    let serum_market = market.optifi_market.serum_market;

    let asset = market.instrument_common.asset;

    let mut market_account = market.serum_account.clone();

    let market_account_info = AccountInfo::new(
        &serum_market,
        false,
        false,
        &mut market_account.lamports,
        &mut market_account.data,
        &mut market_account.owner,
        market_account.executable,
        market_account.rent_epoch,
    );

    let serum_dex_program_id = Pubkey::from_str(SERUM_DEX_PROGRAM_ID).unwrap();

    let serum_market =
        serum_dex::state::Market::load(&market_account_info, &serum_dex_program_id, false).unwrap();

    let mut bids_account = ui_account
        .value
        .decode::<anchor_client::solana_sdk::account::Account>()
        .unwrap();

    let bids_account_info = AccountInfo::new(
        &market.market_pubkeys.bids,
        false,
        true,
        &mut bids_account.lamports,
        &mut bids_account.data,
        &mut bids_account.owner,
        bids_account.executable,
        bids_account.rent_epoch,
    );

    let bids = serum_market
        .load_bids_mut(&bids_account_info)
        .map_err(|err| Error::ProgramError(ProgramError::from(err).into()))
        .unwrap();

    let mut bid_levels: Vec<BookLevel> = vec![];

    for node in bids.traverse().iter() {
        let order = OptifiOrder {
            side: OrderSide::Bid,
            price: u64::from(node.price()) as f64
                / 10_u32.pow(USDC_DECIMALS - asset.get_decimal()) as f64,
            size: node.quantity() as f64 / 10_u32.pow(asset.get_decimal()) as f64,
            client_order_id: node.client_order_id(),
        };
        // println!("{:#?}", order);

        if let Some(level) = bid_levels
            .iter_mut()
            .find(|level| level.price == order.price)
        {
            level.size += order.size;
        } else {
            bid_levels.push(BookLevel {
                price: order.price,
                size: order.size,
            });
        }
    }

    bid_levels
}

#[derive(Debug)]
pub struct Book {
    pub bids: Vec<BookLevel>,
    pub asks: Vec<BookLevel>,
}

#[derive(Debug)]
pub struct BookLevel {
    pub price: f64,
    pub size: f64,
}

pub fn parse_open_orders(market: &Market, orders_ui_account: Response<UiAccount>) {
    let serum_market = market.optifi_market.serum_market;

    let mut market_account = market.serum_account.clone();

    let market_account_info = AccountInfo::new(
        &serum_market,
        false,
        false,
        &mut market_account.lamports,
        &mut market_account.data,
        &mut market_account.owner,
        market_account.executable,
        market_account.rent_epoch,
    );

    let serum_dex_program_id = Pubkey::from_str(SERUM_DEX_PROGRAM_ID).unwrap();

    let serum_market =
        serum_dex::state::Market::load(&market_account_info, &serum_dex_program_id, false).unwrap();

    let mut orders_account = orders_ui_account
        .value
        .decode::<anchor_client::solana_sdk::account::Account>()
        .unwrap();

    let orders_account_info = AccountInfo::new(
        &market.market_pubkeys.bids,
        false,
        true,
        &mut orders_account.lamports,
        &mut orders_account.data,
        &mut orders_account.owner,
        orders_account.executable,
        orders_account.rent_epoch,
    );

    let open_orders = serum_market
        .load_orders_mut(
            &orders_account_info,
            None,
            &serum_dex_program_id,
            None,
            None,
        )
        .unwrap();

    for open_order in open_orders.orders_with_client_ids() {
        println!("client_order_ids: {:#?}", open_order);
    }

    // let if_need_settle = open_orders.native_coin_free > 0 || open_orders.native_pc_free > 0;
}

pub fn parse_user_account(result: &Value) -> Result<UserAccount> {
    parse_user_account_inner(serde_json::from_value(result.clone()).unwrap())
}

pub fn parse_user_account_inner(ui_account: Response<UiAccount>) -> Result<UserAccount> {
    let account = ui_account
        .value
        .decode::<anchor_client::solana_sdk::account::Account>()
        .unwrap();

    UserAccount::try_deserialize(&mut (&account.data as &[u8]))
}

pub fn parse_usdc_account(result: &Value) -> Result<u64> {
    parse_usdc_account_inner(serde_json::from_value(result.clone()).unwrap())
}

pub fn parse_usdc_account_inner(ui_account: Response<UiAccount>) -> Result<u64> {
    let mut account = ui_account
        .value
        .decode::<anchor_client::solana_sdk::account::Account>()
        .unwrap();

    let pubkey = Pubkey::default();

    let account_info = AccountInfo::new(
        &pubkey,
        false,
        true,
        &mut account.lamports,
        &mut account.data,
        &mut account.owner,
        account.executable,
        account.rent_epoch,
    );

    accessor::amount(&account_info)
}