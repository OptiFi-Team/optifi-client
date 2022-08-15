#[cfg(test)]
mod tests {

    use optifi_client::cranker::*;
    use optifi_client::prelude::*;

    #[test]
    fn test_fetch_all_user_accounts() {
        let optifi_client = OptifiClient::new(
            Cluster::Devnet,
            Some("~/.config/solana/optifi.json".to_string()),
            None,
        );

        let user_accounts = optifi_client.fetch_all_user_accounts();

        println!("user_accounts: {:?}", user_accounts.len());
    }
}
