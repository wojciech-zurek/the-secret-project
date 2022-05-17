use std::collections::HashMap;
use nohash_hasher::IntMap;
use crate::account::basic::BasicAccount;
use crate::client::Client;

/// Repository to store client account state
/// This repository is using HashMap/BuildNoHashHasher as hash implementation
/// Client is a valid u16 client ID
pub struct BasicAccountMemoryRepository {
    inner: IntMap<Client, BasicAccount>,
}

impl BasicAccountMemoryRepository {
    pub fn new() -> Self {
        BasicAccountMemoryRepository {
            inner: HashMap::default()
        }
    }

    pub fn find_by_client(&mut self, client: Client) -> &mut BasicAccount {
        self.inner.entry(client).or_insert_with(|| BasicAccount::new(client))
    }

    pub fn get_all_account_iter(&self) -> impl Iterator<Item=&BasicAccount> {
        self.inner.iter().map(|it| it.1)
    }

    pub fn get_all_account_into_iter(self) -> impl Iterator<Item=BasicAccount> {
        self.inner.into_iter().map(|it| it.1)
    }
}