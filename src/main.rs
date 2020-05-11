#[macro_use]
extern crate log;
#[macro_use]
extern crate yaserde_derive;

use env_logger;

mod tokio_onvif;
use tokio_onvif::util;

mod onvif;
use onvif::device_info::OnvifQuery;
use onvif::device_info::OnvifQueryImpl;

#[tokio::main]
async fn main() {
    // let (discovery_rx, discover_tx, stop_tx) = 
    //     util::start_simple_onvif_discovery(std::time::Duration::from_secs(1));
    loop {
        tokio::time::delay_for(std::time::Duration::from_secs(1)).await;
        // discover_tx.lock().await.send(()).await.unwrap();
        // let devices = discovery_rx.lock().await.recv().await.unwrap();

        let devices = util::simple_onvif_discover(std::time::Duration::from_secs(1)).await.unwrap();

        for device_uri in devices.iter() {
            println!("Found device: {:?}", device_uri);
            let onvif_query = OnvifQueryImpl {};
            match onvif_query.get_device_ip_and_mac_address(device_uri).await {
                Ok((ip, mac)) => {
                    println!("Found device ip: {:?}", ip);
                    println!("Found device mac: {:?}", mac);        
                },
                Err(e) => {
                    println!("Error finding ip and mac: {:?}", e);
                }
            };
            match onvif_query.get_device_scopes(device_uri).await {
                Ok(scopes) => {
                    println!("Found device scopes: {:?}", scopes);
                },
                Err(e) => {
                    println!("Error finding scopes: {:?}", e);
                }
            };
            match onvif_query.get_device_profiles(device_uri).await {
                Ok(profiles) => {
                    for profile in profiles {
                        println!("Found device profile: {:?}", profile);
                        match onvif_query.get_device_profile_streaming_uri(device_uri, &profile).await {
                            Ok(streaming_uri) => {
                                println!("Found device streaming uri: {:?}", streaming_uri);
                            },
                            Err(e) => {
                                println!("Error finding streaming uri: {:?}", e);
                            }
                        };
                    }
                },
                Err(e) => {
                    println!("Error finding profiles: {:?}", e);
                }
            };
        }

        
    }
}


// mod std_onvif;
// use std_onvif::util;

// #[tokio::main]
// async fn main() {
//     env_logger::try_init().unwrap();

//     loop {
//         println!("onvif devices found: {:?}", util::simple_onvif_discover(std::time::Duration::from_secs(1)).unwrap());
//         std::thread::sleep(std::time::Duration::from_secs(5));
//     }
// }
