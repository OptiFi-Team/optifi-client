pub use crate::client::*;
use crate::prelude::*;

impl OptifiClient {
    pub fn fetch_all_user_accounts(&self) -> Vec<(Pubkey, UserAccount)> {
        let optifi_exchange_filter = RpcFilterType::Memcmp(Memcmp {
            offset: 8,
            bytes: MemcmpEncodedBytes::Bytes(self.optifi_exchange.to_bytes().to_vec()),
            encoding: None,
        });

        let user_accounts = self
            .program
            .accounts::<UserAccount>(vec![optifi_exchange_filter])
            .unwrap();

        user_accounts
    }

    pub fn initialize_liquidation(&self) {}
}
