use async_std::sync::Arc;
use netmod::Endpoint;
use std::ops::{Deref, DerefMut};
use std::sync::{Mutex, MutexGuard, RwLock};

type Ep = dyn Endpoint + 'static + Send + Sync;

/// A mutable reference to a driver in the DriverMap.
pub struct DriverRef<'a> {
    inner: MutexGuard<'a, Box<Ep>>,
}

impl<'a> From<MutexGuard<'a, Box<Ep>>> for DriverRef<'a> {
    fn from(guard: MutexGuard<'a, Box<Ep>>) -> DriverRef<'a> {
        DriverRef { inner: guard }
    }
}

impl Deref for DriverRef<'_> {
    type Target = Ep;
    fn deref(&self) -> &Ep {
        &(**self.inner)
    }
}

impl DerefMut for DriverRef<'_> {
    fn deref_mut(&mut self) -> &mut Ep {
        &mut (**self.inner)
    }
}

/// A map of available endpoint drivers
///
/// Currently the removing of drivers isn't supported, but it's
/// possible to have the same endpoint in the map multiple times, with
/// unique IDs.
#[derive(Default)]
pub(crate) struct DriverMap {
    map: RwLock<Vec<Mutex<Box<Ep>>>>,
}

impl DriverMap {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Get the length of the driver set
    pub(crate) fn len(&self) -> usize {
        self.map.read().expect("DriverMap RwLock poisoned").len()
    }

    /// Insert a new endpoint to the set of known endpoints
    pub(crate) fn add<E>(&self, ep: E)
    where
        E: Endpoint + 'static + Send + Sync,
    {
        self.map
            .write()
            .expect("DriverMap RwLock poisoned")
            .push(Mutex::new(Box::new(ep)));
    }

    /// Remove the endpoint with the given ID and return it, if it exists
    pub(crate) fn remove(&self, id: usize) -> Option<Box<Ep>> {
        let map = self.map.write().expect("DriverMap RwLock poisoned");
        if id < map.len() {
            Some(
                map.remove(id)
                    .into_inner()
                    .expect("DriverMap Mutex poisoned"),
            )
        } else {
            None
        }
    }

    /// Get mutable access to the endpoint with the given ID, if it exists
    pub(crate) fn get_mut<'a>(&'a self, id: usize) -> Option<DriverRef> {
        self.map
            .read()
            .expect("DriverMap RwLock poisoned")
            .get(id)
            .map(|mutex| mutex.lock().expect("DriverMap Mutex poisoned").into())
    }

    // Add a new interface with a guaranteed unique ID to the map
    // pub(crate) async fn add(&self, ep: impl Endpoint + 'static + Send) {
    //     let (mut map, mut id) = self.map.lock().join(self.curr.lock()).await;
    //     map.insert(*id, Box::new(ep));
    //     *id += 1;
    // }

    // Returns access to the unlocked endpoint collection
    //     pub(crate) async fn inner<'this>(&'this self) -> MutexGuard<'_, EndpointMap> {
    //         self.map.lock().await
    //     }
}
