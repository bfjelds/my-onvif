#[macro_use]
extern crate log;
#[macro_use]
extern crate yaserde_derive;

use env_logger;

mod onvif;
use onvif::util;

#[tokio::main]
async fn main() {
    env_logger::try_init().unwrap();

    futures::join!(tokio::spawn( { async move {
        async {
            trace!("enter my-onvif");
            info!("onvif devices found: {:?}", util::simple_onvif_discover().unwrap());
            trace!("exit my-onvif");
    }.await;
    }}));
}
