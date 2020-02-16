use crate::Message;
use async_std::prelude::*;
use async_std::future;
use async_std::task::{self, Context, Poll};
use async_std::{
    sync::{channel, Arc, Mutex, Receiver, Sender},
};
use netmod::{Frame, SeqBuilder, SeqId};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

type ArcRw<T> = Arc<Mutex<T>>;

/// Local frame collector
pub(crate) struct Collector {
    frags: Sender<(SeqId, Frame)>,
    done: Receiver<Message>,
    in_progress: AtomicUsize
}

impl Collector {
    pub(crate) fn new() -> Arc<Self> {
        let (frags, rx) = channel(5);
        let (tx, done) = channel(1);

        let this = Arc::new(Self { frags, done, in_progress: AtomicUsize::new(0) });
        Arc::clone(&this).run(rx, tx);
        this
    }

    /// Poll the collector for new messages addressed to local users
    pub(crate) async fn completed(&self) -> Message {
        self.done.recv().await.unwrap()
    }

    /// Check whether or not there are completed messages waiting to be dispatched at the moment.
    pub(crate) fn has_completed_messages(&self) -> bool {
        !self.done.is_empty()
    }

    /// Determine the number of messages in the queue
    pub(crate) fn queued(&self) -> usize {
        self.frags.len()
    }

    /// Determine the number of messages being processed by the long-running task and its subtasks
    pub(crate) fn processing(&self) -> usize {
        self.in_progress.load(Ordering::Relaxed)
    }

    /// Compute the total number of messages which need to be processed before completion
    pub(crate) fn in_progress(&self) -> usize {
        self.queued() + self.processing()
    }

    /// Return a future that completes when there are no more fragments in the queue to be processed.
    pub(crate) async fn finish_ingestion(&self) {
        future::poll_fn(|_| {
            if self.in_progress() == 0 {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }).await
    }

    /// Dispatches a long-running task to run the collection logic
    pub(crate) fn run(self: Arc<Self>, inc: Receiver<(SeqId, Frame)>, done: Sender<Message>) {
        let omap: ArcRw<BTreeMap<SeqId, Sender<Frame>>> = Default::default();
        task::spawn(async move {
            while let Some((fid, f)) = inc.recv().await {
                let self_ref = self.clone();
                self_ref.in_progress.fetch_add(1, Ordering::Relaxed);
                println!("Collected a frame from the queue for processing. p: {}", self_ref.in_progress());
                let mut map = omap.lock().await;

                // Check if a handler was already spawned for SeqId
                if map.contains_key(&fid) {
                    map.get(&fid).unwrap().send(f).await;
                } else {
                    // Otherwise we spawn the handler
                    let done = done.clone();
                    let (tx, rx) = channel(1);
                    map.insert(fid.clone(), tx);
                    let map = Arc::clone(&omap);

                    task::spawn(async move {
                        let mut buf: Vec<Frame> = vec![];
                        while let Some(f) = rx.recv().await {
                            self_ref.in_progress.fetch_sub(1, Ordering::Relaxed);
                            println!("Processed a frame in the subtask for {}. p: {}", &fid, self_ref.in_progress());
                            match join_frames(&mut buf, f) {
                                // If the sequence was complete, clean up handlers
                                Some(msg) => {
                                    done.send(msg).await;
                                    map.lock().await.remove(&fid);
                                    break;
                                }
                                None => { println!("Breaking from rx.recv() loop in the subtask for {}. p: {}", &fid, self_ref.in_progress()); continue },
                            }
                        }
                    });
                }
            }
            println!("Finished with main task loop. p: {}", self.in_progress());
        });
    }

    /// Enqueue a frame to be desequenced
    pub(crate) async fn queue(&self, f: Frame) {
        let seqid = f.seq.seqid;
        self.frags.send((seqid, f)).await;
    }
}

/// Utility function that uses the SeqBuilder to rebuild Sequence
fn join_frames(buf: &mut Vec<Frame>, new: Frame) -> Option<Message> {
    // Insert the frame
    buf.push(new);

    // Sort by sequence numbers
    buf.sort_by(|a, b| a.seq.num.cmp(&b.seq.num));

    // The last frame needs to point to `None`
    if buf.last().unwrap().seq.next.is_some() {
        return None;
    }
    // Test inductive sequence number property
    if buf.iter().enumerate().fold(true, |status, (i, frame)| {
        status && (frame.seq.num == i as u32)
    }) {
        let id = buf[0].seq.seqid;
        let sender = buf[0].sender;
        let recipient = buf[0].recipient;
        let payload = SeqBuilder::restore(buf);

        Some(Message {
            id,
            sender,
            recipient,
            payload,
            signature: vec![],
        })
    } else {
        None
    }
}

#[cfg(test)]
use identity::Identity;
#[cfg(test)]
use netmod::Recipient;

#[test]
fn join_frame_simple() {
    let sender = Identity::random();
    let resp = Identity::random();
    let seqid = Identity::random();

    let mut seq = SeqBuilder::new(sender, Recipient::User(resp), seqid)
        .add((0..10).into_iter().collect())
        .add((10..20).into_iter().collect())
        .add((20..30).into_iter().collect())
        .build();

    // The function expects a filling buffer
    let mut buf = vec![];

    assert!(join_frames(&mut buf, seq.remove(0)) == None);
    assert!(join_frames(&mut buf, seq.remove(1)) == None); // Insert out of order
    assert!(join_frames(&mut buf, seq.remove(0)).is_some());
}

#[test]
fn join_frames_in_order_async() {
    println!("Running async frame test. Here, `p` represents the number of frames not yet processed.");
    let (sender, recipient, seqid) = (Identity::random(), Identity::random(), Identity::random());
    let mut seq = SeqBuilder::new(sender, Recipient::User(recipient), seqid)
        .add(vec![0, 1, 2, 3])
        .add(vec![4, 5, 6, 7])
        .add(vec![8, 9, 10, 11])
        .add(vec![12, 13, 14, 15])
        .build();

    task::block_on(async {
        let collector = Collector::new();
        let (_, incoming) = channel(1);
        let (messages, _) = channel(1);
        collector.clone().run(incoming, messages);

        for frame in seq {
            collector.queue(frame).await;
            println!("Queued frame. p: {}", collector.in_progress());
        }

        println!("All frames queued. p: {}", collector.in_progress());

        collector.finish_ingestion().timeout(std::time::Duration::from_millis(100)).await.expect("Failed to complete ingestion.");

        println!("Processed frames. p: {}", collector.in_progress());

        assert!(collector.has_completed_messages());
    });
}
