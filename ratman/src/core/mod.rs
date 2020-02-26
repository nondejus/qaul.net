//! Routing core components
//!
//! In previous designs (both code and docs) this was a single
//! component. This has proven to be a hard to maintain approach, so
//! instead the core has been split into several parts.

mod collector;
mod dispatch;
mod drivers;
mod journal;
mod routes;
mod switch;

pub(self) use collector::Collector;
pub(self) use dispatch::Dispatch;
pub(self) use drivers::DriverMap;
pub(self) use journal::Journal;
pub(self) use routes::{EpTargetPair, RouteTable, RouteType};
pub(self) use switch::Switch;

use crate::{Endpoint, Identity, Message, Result, Slicer, Error};
use async_std::sync::Arc;

/// The Ratman routing core interface
///
/// The core handles sending, receiving and storing frames that can't
/// be delivered at that time (delay-tolerance).
pub(crate) struct Core {
    collector: Arc<Collector>,
    dispatch: Arc<Dispatch>,
    journal: Arc<Journal>,
    routes: Arc<RouteTable>,
    switch: Arc<Switch>,
    drivers: Arc<DriverMap>,
}

impl Core {
    /// Initialises, but doesn't run the routing core
    pub(crate) fn init() -> Self {
        let drivers = DriverMap::new();
        let routes = RouteTable::new();
        let journal = Journal::new();

        let dispatch = Dispatch::new(Arc::clone(&routes), Arc::clone(&drivers));
        let collector = Collector::new();

        let switch = Switch::new(
            Arc::clone(&routes),
            Arc::clone(&journal),
            Arc::clone(&dispatch),
            Arc::clone(&collector),
            Arc::clone(&drivers),
        );

        Self {
            dispatch,
            routes,
            collector,
            journal,
            switch,
            drivers,
        }
    }

    /// Asynchronously runs all routing core subroutines
    ///
    /// **Note**: currently it's not possible to gracefully shut down
    /// the core subsystems!
    pub(crate) fn run(&self) {
        Arc::clone(&self.switch).run();
        Arc::clone(&self.journal).run();
    }

    /// Asynchronously send a Message
    pub(crate) async fn send(&self, msg: Message) {
        let frames = Slicer::slice(0, msg);

        for f in frames.into_iter() {
            self.dispatch.send(f).await;
        }
    }

    /// Poll for the incoming Message
    pub(crate) async fn next(&self) -> Message {
        self.collector.completed().await
    }

    /// Check if an Id is present in the routing table
    pub(crate) async fn known(&self, id: Identity, local: bool) -> Result<()> {
        if local {
            self.routes.local(id).await
        } else {
            self.routes.resolve(id).await.map_or(Err(Error::NoUser), |_| Ok(()))
        }
    }

    /// Returns users that were newly discovered in the network
    pub(crate) async fn discover(&self) -> Identity {
        self.routes.discover().await
    }
    
    /// Insert a new endpoint
    pub(crate) async fn add_ep(&self, ep: impl Endpoint + 'static + Send + Sync) -> usize {
        self.drivers.add(ep).await
    }

    /// Remove an endpoint
    pub(crate) async fn rm_ep(&self, id: usize) {
        self.drivers.remove(id).await;
    }
    
    /// Add a local user endpoint
    pub(crate) async fn add_local(&self, id: Identity) -> Result<()> {
        self.routes.add_local(id).await
    }

    /// Remove a local user endpoint
    pub(crate) async fn rm_local(&self, id: Identity) -> Result<()> {
        self.routes.delete(id).await
    }
}
