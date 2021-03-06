//! The frame/message collector module
//!
//! The collector module is a bit more complicated than other modules,
//! because of the layers of state and control inversion it has to
//! contend with.
//!
//! It would be possible to do all in one file, but it would quickly
//! become too complicated, and unmaintainable.  Instead, this module
//! splits the code into three sections: the state, the worker, and
//! the manager.  The former two exploit the latter for profit.
//!
//! The manager is exposed from this module as `Collector`, so that
//! the routing core and other modules don't have to care about the
//! inner workings.  The state mostly provides a way to create and
//! yield workers, that are being polled by the manager.  The workers
//! themselves have very little control over their environment, only
//! getting access to the state manager to ask for more work, and then
//! making themselves redundant by handing in their finished messages.

use crate::Message;
use async_std::{
    sync::{Arc, Mutex},
    task,
};
use netmod::{Frame, SeqId};
use std::collections::BTreeMap;

pub(self) type Locked<T> = Arc<Mutex<T>>;

mod state;
pub(self) use state::State;

mod worker;
pub(self) use worker::Worker;

/// The main collector management structure and API facade
pub(crate) struct Collector {
    state: Arc<State>,
    workers: Locked<BTreeMap<SeqId, Arc<Worker>>>,
}

impl Collector {
    /// Create a new collector
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Arc::new(State::new()),
            workers: Default::default(),
        })
    }

    /// Queue a new frame to collect
    ///
    /// This function can spawn new workers when needed
    #[cfg(test)]
    pub(crate) async fn queue(&self, seq: SeqId, f: Frame) {
        self.state.queue(seq, f).await;

        let mut map = self.workers.lock().await;
        if !map.contains_key(&seq) {
            map.insert(seq, Arc::new(Worker::new(seq, Arc::clone(&self.state))));
        }
    }

    /// Queue the work, and spawn a worker if required
    pub(crate) async fn queue_and_spawn(&self, seq: SeqId, f: Frame) {
        self.state.queue(seq, f).await;

        let mut map = self.workers.lock().await;
        if !map.contains_key(&seq) {
            map.insert(seq, Arc::new(Worker::new(seq, Arc::clone(&self.state))));
            self.spawn_worker(seq).await;
        }
    }
    
    /// Get any message that has been completed
    pub(crate) async fn completed(&self) -> Message {
        self.state.completed().await
    }

    #[cfg(test)]
    pub(crate) async fn num_queued(&self) -> usize {
        self.state.num_queued().await
    }

    #[cfg(test)]
    pub(crate) async fn num_completed(&self) -> usize {
        self.state.num_completed().await
    }

    /// Get raw access to a worker poll cycle, for testing purposes
    #[cfg(test)]
    async fn get_worker(&self, seq: SeqId) -> Arc<Worker> {
        Arc::clone(&self.workers.lock().await.get(&seq).unwrap())
    }

    /// Spawn an async task runner for a worker
    async fn spawn_worker(&self, seq: SeqId) {
        let workers = Arc::clone(&self.workers);

        let worker = {
            let map = workers.lock().await;
            Arc::clone(&map.get(&seq).unwrap())
        };

        task::spawn(async move {
            // This loop breaks when the worker is done
            while let Some(()) = worker.poll().await {}

            // Then remove it
            let mut map = workers.lock().await;
            map.remove(&seq).unwrap();
        });
    }
}


#[cfg(test)]
use crate::Identity;

#[test]
fn queue_one() {
    use netmod::Recipient;
    use crate::Slicer;
    
    let (sender, recipient, id) = (Identity::random(), Identity::random(), Identity::random());
    let mut seq = Slicer::slice(128, Message {
        id,
        sender,
        recipient: Recipient::User(recipient),
        payload: vec![0, 1, 2, 3, 1, 3, 1, 2, 1, 3, 3, 7],
        sign: vec![0, 1],
    });
    
    assert_eq!(seq.len(), 1);
    let frame = seq.remove(0);
    let seqid = id;

    task::block_on(async move {
        let c = Collector::new();

        // There is one queued frame
        c.queue(seqid, frame).await;
        assert!(c.num_queued().await == 1);

        // After we handle it, the worker can die
        let w = c.get_worker(seqid).await;
        assert!(w.poll().await == None);

        // Now get the finished message
        assert!(c.completed().await.id == seqid);
    });
}

#[test]
fn queue_many() {
    use netmod::Recipient;
    use crate::Slicer;
    
    let (sender, recipient, id) = (Identity::random(), Identity::random(), Identity::random());
    let seq = Slicer::slice(8, Message {
        id,
        sender,
        recipient: Recipient::User(recipient),
        payload: vec![0, 1, 2, 3, 1, 3, 1, 2, 1, 3, 3, 7],
        sign: vec![],
    });
    
    let seqid = id;
    let len = seq.len();
    assert_eq!(len, 2);
    
    task::block_on(async move {
        let c = Collector::new();

        for f in seq {
            c.queue(seqid, f).await;
        }

        // There is n queued frames
        assert!(c.num_queued().await == 2);

        let w = c.get_worker(seqid).await;

        // We can twice three times before the worker dies
        assert!(w.poll().await == Some(()));
        assert!(w.poll().await == None);

        // Now get the finished message
        assert!(c.completed().await.id == seqid);
    });
}


#[cfg(test)]
fn queue_test(num: usize) -> Arc<Collector> {
    use netmod::{Recipient, SeqBuilder};

    let (sender, recipient, seqid) = (Identity::random(), Identity::random(), Identity::random());
    let seq = SeqBuilder::new(sender, Recipient::User(recipient), seqid)
        .add(vec![1, 3, 1, 2])
        .build();

    task::block_on(async move {
        let col = Collector::new();
        let mut vec = vec![];

        for f in seq.into_iter().cycle().take(num) {
            let seqid = Identity::random();
            col.queue(seqid, f).await;
            vec.push(seqid);
        }

        assert!(col.num_queued().await == num);

        for seqid in vec {
            col.spawn_worker(seqid).await;
        }

        // Return the collector to be re-used
        col
    })
}

#[test]
fn queue_1000() {
    queue_test(1000);
}

#[test]
fn queue_10000() {
    queue_test(10000);
}

// This test is a bit bleh because each message sequence is only 1
// frame long.  There should be better test at generating frame
// sequences, but we can always add that later.
#[cfg(test)]
fn queue_and_collect_test(num: usize) {
    use std::time::Duration;
    let col = queue_test(num);

    task::block_on(async move {
        while col.num_completed().await != num {
            println!("Completed: {}", col.num_completed().await);
            task::sleep(Duration::from_millis(100)).await;
        }

        let mut vec = vec![];

        for _ in 0..num {
            vec.push(col.completed().await);
        }

        assert!(vec.len() == num);
    });
}

#[test]
fn queue_and_collect_1000() {
    queue_and_collect_test(1000);
}

#[test]
fn queue_and_collect_10000() {
    queue_and_collect_test(10000);
}

#[test]
fn queue_and_collect_100000() {
    queue_and_collect_test(100000);
}

#[test]
#[ignore]
fn queue_and_collect_1000000() {
    queue_and_collect_test(1000000);
}
