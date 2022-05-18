use nohash_hasher::IntMap;
use crate::account::wrap::WrapAccount;
use crate::account::basic::BasicAccount;
use crate::client::Client;

/// Repository to store client account state
/// This repository is using HashMap/BuildNoHashHasher as hash implementation
/// Client is a valid u16 client ID
pub struct WrapAccountMemoryRepository {
    inner: IntMap<Client, WrapAccount>,
}

impl WrapAccountMemoryRepository {
    pub fn new() -> Self {
        WrapAccountMemoryRepository {
            inner: IntMap::default()
        }
    }

    pub fn find_by_client(&mut self, client: Client) -> &mut WrapAccount {
        self.inner.entry(client).or_insert_with(|| WrapAccount::new(client))
    }

    pub fn get_all_account_into_iter(self) -> impl Iterator<Item=BasicAccount> {
        self.inner.into_iter().map(|it| {
            it.1.into_account()
        })
    }
}