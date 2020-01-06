use libqaul::*;
use libqaul_http::*;
use std::sync::Arc;

fn main() {
    let qaul = Arc::new(Qaul::start());
    qaul.user_create("acab").expect("Failed to create test user!");
    
    let _server = ServerBuilder::new(qaul.clone()).start("0.0.0.0:9090")
        .expect("Failed to start qaul.net API server (0.0.0.0:9090)!");

    #[allow(deprecated)]
    loop { std::thread::sleep_ms(500) };
}
