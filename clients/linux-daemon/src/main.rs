use ratman::Router;
use libqaul::{Qaul, messages::Mode};
use netmod_udp::Endpoint as UdpEndpoint;
use std::sync::Arc;
use async_std::task;
use async_std::prelude::*;

const IPC_SERVICE_NAME: &'static str = "qaul.qauld.server-ipc";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = UdpEndpoint::spawn("0.0.0.0:12226");
    let router = Router::new();
    router.add_endpoint(Arc::try_unwrap(server).unwrap_or_else(|_| panic!("Couldn't get endpoint.")));
    let core = Qaul::new(router);

    let server_user = core.users().create("REPLACE ME")?;
    core.services().register(IPC_SERVICE_NAME)?;
    core.users().update(server_user.clone(), |p| { p.services.insert(IPC_SERVICE_NAME.into()); })?;
    let mut subscription = core.messages().subscribe(server_user.clone(), IPC_SERVICE_NAME, vec![])?;

    let hypoth_user = core.users().create("REPLACE ME TOO")?;
    let server_identity = server_user.clone().0;

    task::block_on(async {
        println!("Sending message...");
        let r = core.messages().send(hypoth_user.clone(), Mode::Std(server_identity), IPC_SERVICE_NAME, vec![], "TEST TEST TEST".as_bytes().into()).await.expect("Failed to send message.");
        println!("Successfully sent message {}.", r);
        while let Some(msg) = subscription.next().await {
            println!("{:?}", msg);
        }
    });

    Ok(())
}
