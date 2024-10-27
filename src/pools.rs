use serde::{Deserialize, Serialize};
// Deserialisation requires lifetime 'de to outlive 'static.
// This is why private is converted to public which is deserializable.
#[derive(Debug, Copy, Clone)]
pub struct PrivatePool {
    pub pubkey: &'static str,
    pub url: &'static str,
    pub is_active: bool,
}

impl PrivatePool {
    pub fn getpool(self) -> Pool {
        Pool {
            pubkey: self.pubkey.to_string(),
            url: Some(self.url.to_string()),
            pool_is_active: Some(self.is_active),
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Pool {
    pub pubkey: String,
    pub url: Option<String>,
    pub pool_is_active: Option<bool>,
}

const CRYMEL: PrivatePool = PrivatePool {
    pubkey: "pcs137vfy28eytanejvp5ku4grgk3q8cfd5wuknrcp",
    url: "https://particl1.crymel.icu/",
    is_active: true,
};

const COLDSTAKINGPOOL: PrivatePool = PrivatePool {
    pubkey: "pcs19453kf98kz47yktqv7x36j39xa07mtvqx8evse",
    url: "https://coldstakingpool.com/",
    is_active: true,
};

pub const POOLS: [PrivatePool; 2] = [
    CRYMEL,
    COLDSTAKINGPOOL,
];
