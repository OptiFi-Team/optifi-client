#[cfg(test)]
mod tests {

    use optifi_client::client::OptifiClient;
    use optifi_client::prelude::*;

    #[test]
    fn test_initialize_user_account() {
        let optifi_client = OptifiClient::new(
            Cluster::Devnet,
            Some("~/.config/solana/optifi.json".to_string()),
            None,
        );

        let signature = optifi_client.initialize_user_account();

        println!("signature: {:?}", signature);
    }

    #[test]
    fn test_load_optifi_exchange() {
        let mut optifi_client = OptifiClient::new(
            Cluster::Mainnet,
            Some("~/.config/solana/optifi.json".to_string()),
            None,
        );

        optifi_client.load_optifi_exchange();

        println!("exchange: {:#?}", optifi_client.account.optifi_exchange);
    }

    #[test]
    fn test_load_user_account() {
        let mut optifi_client = OptifiClient::new(
            Cluster::Devnet,
            Some("~/.config/solana/optifi.json".to_string()),
            None,
        );

        optifi_client.load_user_account();

        println!("user_account pubkey: {:#?}", optifi_client.user_account);

        println!("user_account: {:#?}", optifi_client.account.user_account);
    }

    #[test]
    fn test_load_markets() {
        let mut optifi_client = OptifiClient::new(
            Cluster::Mainnet,
            // Cluster::Custom("https://optifil-develope-610c.devnet.rpcpool.com/2fc9e4ee-7e7b-47c5-a9af-a3a4dc0f79c9".to_string(), "".to_string()),
            Some("~/.config/solana/optifi.json".to_string()),
            None,
        );

        optifi_client.load_optifi_exchange();

        // let start = Instant::now();

        optifi_client.load_markets();

        // println!("Time for load_markets: {:?}", start.elapsed());

        println!("markets: {:#?}", optifi_client.account.markets);
    }

    #[test]
    fn test_deposit() {
        let optifi_client = OptifiClient::new(
            Cluster::Devnet,
            Some("~/.config/solana/optifi.json".to_string()),
            None,
        );

        let signature = optifi_client.deposit(100.).unwrap();

        println!("signature: {:#?}", signature);
    }

    #[test]
    fn test_withdraw() {
        let optifi_client = OptifiClient::new(
            Cluster::Devnet,
            Some("~/.config/solana/optifi.json".to_string()),
            None,
        );

        let signature = optifi_client.withdraw(100.).unwrap();

        println!("signature: {:#?}", signature);
    }

    #[test]
    fn test_initialize_user_on_market() {
        let mut optifi_client = OptifiClient::new(
            Cluster::Devnet,
            Some("~/.config/solana/optifi.json".to_string()),
            None,
        );

        optifi_client.load_optifi_exchange();

        optifi_client.load_markets();

        let signature = optifi_client
            .initialize_user_on_market(&optifi_client.account.markets[0])
            .ok();

        println!("signature: {:#?}", signature);
    }

    #[test]
    fn test_set_delegation() {
        let mut optifi_client = OptifiClient::new(
            Cluster::Devnet,
            Some("~/.config/solana/optifi.json".to_string()),
            None,
        );

        optifi_client.load_user_account();

        // Wallet and cluster params.
        let delegatee_wallet_path = "~/.config/solana/delegatee.json".to_string();

        let delegatee = read_keypair_file(&*shellexpand::tilde(&delegatee_wallet_path))
            .expect("Example requires a keypair file");

        let signature = optifi_client.set_delegation(Some(delegatee.pubkey()));

        println!("signature: {:#?}", signature);
    }

    #[test]
    fn test_place_order() {
        let optifi_client = OptifiClient::initialize(
            Cluster::Devnet,
            Some("~/.config/solana/optifi.json".to_string()),
            None,
        );

        println!("market: {:#?}", &optifi_client.account.markets[1]);

        let signature = optifi_client
            .place_order(
                &optifi_client.account.markets[0],
                OrderSide::Bid,
                20.,
                4.1,
                OrderType::ImmediateOrCancel,
            )
            .unwrap();

        println!("signature: {:#?}", signature);
    }

    #[test]
    fn test_place_order_with_delegation() {
        let delegator = read_keypair_file(&*shellexpand::tilde("~/.config/solana/optifi.json"))
            .expect("Example requires a keypair file");

        let optifi_client = OptifiClient::initialize(
            Cluster::Devnet,
            Some("~/.config/solana/delegatee.json".to_string()),
            Some(delegator.pubkey()),
        );

        // println!("market: {:#?}", &optifi_client.account.markets[0]);

        let signature = optifi_client
            .place_order(
                &optifi_client.account.markets[0],
                OrderSide::Bid,
                1.,
                1.,
                OrderType::Limit,
            )
            .unwrap();

        println!("signature: {:#?}", signature);
    }

    #[test]
    fn test_settle_order() {
        let optifi_client = OptifiClient::initialize(
            Cluster::Devnet,
            Some("~/.config/solana/optifi.json".to_string()),
            None,
        );

        let signature = optifi_client
            .settle_order(&optifi_client.account.markets[0])
            .unwrap();

        println!("signature: {:#?}", signature);
    }

    #[test]
    fn test_load_open_orders() {
        let optifi_client = OptifiClient::initialize(
            Cluster::Devnet,
            Some("~/.config/solana/optifi.json".to_string()),
            None,
        );

        let orders = optifi_client.load_open_orders(&optifi_client.account.markets[0]);

        println!("{:#?}", orders);
    }

    #[test]
    fn test_load_order_book() {
        let mut optifi_client = OptifiClient::new(
            Cluster::Devnet,
            Some("~/.config/solana/optifi.json".to_string()),
            None,
        );

        optifi_client.load_optifi_exchange();

        optifi_client.load_markets();

        println!("{:#?}", &optifi_client.account.markets.last().unwrap());

        let order_book =
            optifi_client.load_order_book(&optifi_client.account.markets.last().unwrap());

        println!("{:#?}", &order_book);
    }

    #[test]
    fn test_cancel_order_with_delegation() {
        let optifi_client = OptifiClient::initialize(
            Cluster::Devnet,
            Some("~/.config/solana/id.json".to_string()),
            Some(Pubkey::from_str("GRLYbdHJEtC3yu48cPbHwadqfek2Y3C75CLXsGmvoqE1").unwrap()),
        );

        let open_orders = optifi_client.load_open_orders(&optifi_client.account.markets[0]);

        println!("open_orders: {:#?}", open_orders);

        let signature = optifi_client
            .cancel_order(
                &optifi_client.account.markets[0],
                open_orders[0].side,
                open_orders[0].client_order_id,
            )
            .unwrap();

        println!("signature: {:#?}", signature);
    }

    #[test]
    fn test_cancel_order() {
        let optifi_client = OptifiClient::initialize(
            Cluster::Devnet,
            Some("~/.config/solana/optifi.json".to_string()),
            None,
        );

        let open_orders = optifi_client.load_open_orders(&optifi_client.account.markets[0]);

        println!("open_orders: {:#?}", open_orders);

        let signature = optifi_client
            .cancel_order(
                &optifi_client.account.markets[0],
                open_orders[0].side,
                open_orders[0].client_order_id,
            )
            .unwrap();

        println!("signature: {:#?}", signature);
    }

    #[test]
    fn test_cancel_all_order() {
        let optifi_client = OptifiClient::initialize(
            Cluster::Devnet,
            Some("~/.config/solana/optifi.json".to_string()),
            None,
        );

        optifi_client.cancel_all_order(&optifi_client.account.markets[0]);
    }

    #[test]
    fn test_subscribe_ask() {
        let optifi_client = OptifiClient::initialize(
            Cluster::Devnet,
            Some("~/.config/solana/optifi.json".to_string()),
            None,
        );

        println!("{:#?}", &optifi_client.account.markets[0]);

        optifi_client.subscribe_ask(&optifi_client.account.markets[0]);
    }

    #[test]
    fn load_other_user() {
        let optifi_client = OptifiClient::new(
            Cluster::Devnet,
            Some("~/.config/solana/optifi.json".to_string()),
            None,
        );

        let user = Pubkey::from_str("H1HDXTwT8SStYffNkAqVzy3p9PEKoeg94nUhYr4YkvQx").unwrap();

        let (user_account_key, ..) =
            get_user_account_pda(&optifi_client.optifi_exchange, &user, &optifi_cpi::id());

        let account: UserAccount = optifi_client.program.account(user_account_key).unwrap();

        println!("{:#?}", account);
    }

    #[test]
    fn test_subscribe_open_orders() {
        let optifi_client = OptifiClient::initialize(
            Cluster::Devnet,
            Some("~/.config/solana/optifi.json".to_string()),
            None,
        );

        println!("{:#?}", &optifi_client.account.markets[0]);

        optifi_client.subscribe_open_orders(&optifi_client.account.markets[0]);
    }

    #[test]
    fn test_subscribe_user_account() {
        let mut optifi_client = OptifiClient::new(
            Cluster::Devnet,
            Some("~/.config/solana/optifi.json".to_string()),
            None,
        );

        optifi_client.load_user_account();

        optifi_client.subscribe_user_account();
    }
}
