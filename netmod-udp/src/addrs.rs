//! Address resolution table module

use async_std::{
    net::IpAddr,
    sync::{Arc, RwLock},
};
use std::collections::BTreeMap;

/// A small utility that creates sequential IDs
struct IdMaker {
    last: Arc<RwLock<u16>>,
}

impl IdMaker {
    async fn curr(&self) -> u16 {
        *self.last.read().await
    }

    async fn incr(&self) -> &Self {
        *self.last.write().await += 1;
        self
    }
}

pub(crate) struct AddrTable {
    factory: IdMaker,
    ips: Arc<RwLock<BTreeMap<u16, IpAddr>>>,
    ids: Arc<RwLock<BTreeMap<IpAddr, u16>>>,
}

impl AddrTable {
    /// Create a new address lookup table
    pub(crate) fn new() -> Self {
        Self {
            factory: IdMaker {
                last: Default::default(),
            },
            ips: Default::default(),
            ids: Default::default(),
        }
    }

    /// Insert a given IP into the table, returning it's ID
    ///
    /// Topology changes are handled additively, because it's not
    /// possible to find out what previous IP a node had, without
    /// performing deep packet inspection and looking at certain
    /// Identity information.  As such, this table can only grow.
    pub(crate) async fn set(&self, ip: IpAddr) -> u16 {
        let id = self.factory.incr().await.curr().await;
        self.ips.write().await.insert(id, ip).unwrap();
        self.ids.write().await.insert(ip, id).unwrap();
        id
    }

    /// Get the ID for a given IP address
    pub(crate) async fn id(&self, ip: &IpAddr) -> Option<u16> {
        self.ids.read().await.get(ip).cloned()
    }

    /// Get the IP for a given internal ID
    pub(crate) async fn ip(&self, id: u16) -> Option<IpAddr> {
        self.ips.read().await.get(&id).cloned()
    }

    pub(crate) async fn all(&self) -> Vec<IpAddr> {
        self.ips.read().await.values().cloned().collect()
    }
}
