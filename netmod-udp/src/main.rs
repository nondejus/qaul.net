//! netmod-udp is a UDP overlay for RATMAN

use async_std::{net::UdpSocket, task, io};
use identity::Identity;
use netmod::{Sequence, Recipient, Endpoint, Frame, Error};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    net::IpAddr,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use conjoiner;

enum UdpCommand {
    /// Used to announce a netmod endpoint via broadcasts
    Announce,
    /// Send an ID announcement to known UDP endpoints
    Id(Identity),
}

/// An internal envelope that is used as a transfer protocol
enum UdpEnvelope {
    /// A tunneled data payload
    Data(Vec<u8>),
    /// An overlay command payload
    Internal(UdpCommand),
}

/// Represents an IP network tunneled via UDP
#[derive(Clone)]
pub struct UdpEndpoint {
    sock: Arc<Mutex<UdpSocket>>,
    ips: Arc<Mutex<BTreeSet<IpAddr>>>,
    nat: Arc<Mutex<BTreeMap<Identity, IpAddr>>>,
    inbox: Arc<Mutex<VecDeque<Frame>>>,
}

impl UdpEndpoint {
    /// Create a new UDP endpoint handler at the given address.
    pub fn with_addr(addr: &str) -> io::Result<Self> {
        let socket = task::block_on(async { UdpSocket::bind(addr).await })?;
        socket.set_broadcast(true);
        let endpoint = Self {
            sock: Arc::new(Mutex::new(socket)),
            ips: Default::default(),
            nat: Default::default(),
            inbox: Default::default(),
        };
        let mut endpoint_handle = endpoint.clone();
        thread::spawn(move ||{
            let mut frame = None;
            loop {
                // TODO: How do we deal with larger sizes? We shouldn't have to because
                // the size_hint says 4k but we might have to.
                let mut buf = vec![0; 8192];
                task::block_on(async { endpoint_handle.sock.lock().expect("Socket mutex poisoned")
                    .recv(&mut buf).await
                    .expect("Could not recv() on socket") });
                // TODO: This obviously shouldn't panic, we should do better error
                // handling here.
                frame = Some(conjoiner::deserialise(&buf).expect("couldn't deserialise. error: "));
            }

            if let Some(f) = frame {
                endpoint_handle.inbox.lock().expect("Inbox mutex poisoned").push_back(f);
                frame = None;
            }
        });
        Ok(endpoint.clone())
    }
}

impl Endpoint for UdpEndpoint {
    fn size_hint(&self) -> usize { 4096 }
    fn send(&mut self, frame: Frame) -> Result<(), Error> { 
        let peer_address = match frame.recipient {
            Recipient::User(ref identity) => { unimplemented!() },
            Recipient::Flood => { "255.255.255.255:1722" }
        };
        let buffer = dbg!(conjoiner::serialise(&frame).expect("Could not serialise frame. error:"));
        let result = task::block_on(self.sock.lock().expect("Socket mutex poisoned")
            .send_to(&buffer, peer_address));
        match result {
            Ok(_) => Ok(()),
                // TODO: Maybe there is a more appropriate error?
            Err(e) => Err(Error::ConnectionLost)
        }
    }

    fn poll(&mut self) -> Result<Option<Frame>, Error> {
        let mut inbox = self.inbox.lock().expect("Inbox mutex poisoned");
        Ok(inbox.pop_front())
    }

    fn listen(&mut self, mut handler: Box<dyn FnMut(Frame) -> Result<(), Error>>) -> Result<(), Error> {
        unimplemented!()
    }
}

fn main() {
    task::block_on(async {
        println!("Build socsender");
        let mut socsender = UdpEndpoint::with_addr("0.0.0.0:1721").unwrap();        
        println!("Build socreceiver");
        let mut socreceiver = UdpEndpoint::with_addr("0.0.0.0:1722").unwrap();
        let mut seq = Sequence::new(Identity::truncate(&vec![0; 16]), Recipient::Flood, [0; 16]);
        let frames = seq.add(b"Hello, UDP universe.".to_vec()).build();
        println!("Sending a frame");
        socsender.send(frames[0].clone());
        println!("Waiting to get a frame");
        thread::sleep(Duration::from_millis(1000));
        dbg!(socreceiver.poll())
    });
}


// /// The following is the server code running on my NAS
// fn main() {
//     task::block_on(async {
//         let socket = UdpSocket::bind("0.0.0.0:1312").await.unwrap();
//         let addr = socket.local_addr().unwrap();
//         // socket.connect("10.7.1.123:1312").await.unwrap();
//         dbg!(addr);
        
//         let mut buf = vec![0u8; 1024];
//         loop {
//             dbg!(socket.recv(&mut buf).await.unwrap());
//             dbg!(&buf);
//         }
//     });
// }