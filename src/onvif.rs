mod to_serialize {
    use super::common::*;
    use std::io::Write;
    use yaserde::YaSerialize;

    #[derive(Default, PartialEq, Debug, YaSerialize)]
    #[yaserde(prefix = "s", namespace = "s: http://www.w3.org/2003/05/soap-envelope")]
    pub struct Envelope {
        #[yaserde(prefix = "s", rename = "Header")]
        pub header: Header,

        #[yaserde(prefix = "s", rename = "Body")]
        pub body: Body,
    }

    #[derive(Default, PartialEq, Debug, YaSerialize)]
    #[yaserde(
        prefix = "s",
        namespace = "s: http://www.w3.org/2003/05/soap-envelope",
        namespace = "d: http://schemas.xmlsoap.org/ws/2005/04/discovery"
    )]
    pub struct Body {
        #[yaserde(prefix = "d", rename = "Probe")]
        pub probe: Probe,
    }

    #[derive(Default, PartialEq, Debug, YaSerialize)]
    #[yaserde(
        prefix = "s",
        namespace = "s: http://www.w3.org/2003/05/soap-envelope",
        namespace = "w: http://schemas.xmlsoap.org/ws/2004/08/addressing"
    )]
    pub struct Header {
        #[yaserde(prefix = "w", rename = "MessageID")]
        pub message_id: String,

        #[yaserde(prefix = "w", rename = "To")]
        pub reply_to: String,

        #[yaserde(prefix = "w", rename = "Action")]
        pub action: String,
    }
}

mod to_deserialize {
    use super::common::*;
    use std::io::Read;
    use yaserde::YaDeserialize;

    #[derive(Default, PartialEq, Debug, YaDeserialize)]
    #[yaserde(prefix = "s", namespace = "s: http://www.w3.org/2003/05/soap-envelope")]
    pub struct Envelope {
        #[yaserde(prefix = "s", rename = "Header")]
        pub header: Header,

        #[yaserde(prefix = "s", rename = "Body")]
        pub body: Body,
    }

    #[derive(Default, PartialEq, Debug, YaDeserialize)]
    #[yaserde(
        prefix = "s",
        namespace = "s: http://www.w3.org/2003/05/soap-envelope",
        namespace = "d: http://schemas.xmlsoap.org/ws/2005/04/discovery"
    )]
    pub struct Body {
        #[yaserde(prefix = "d", rename = "ProbeMatches")]
        pub probe_matches: ProbeMatches,
    }

    #[derive(Default, PartialEq, Debug, YaDeserialize)]
    #[yaserde(
        prefix = "s",
        namespace = "s: http://www.w3.org/2003/05/soap-envelope",
        namespace = "w: http://schemas.xmlsoap.org/ws/2004/08/addressing"
    )]
    pub struct Header {
        #[yaserde(prefix = "w", rename = "RelatesTo")]
        pub relates_to: String,
    }
}

mod common {
    use std::io::{Read, Write};
    use yaserde::{YaDeserialize, YaSerialize};

    #[derive(PartialEq, Debug, YaDeserialize, YaSerialize)]
    pub enum ProbeType {
        #[yaserde(rename = "wsdl:Device")]
        Device,
        #[yaserde(rename = "wsdl:NetworkVideoTransmitter")]
        NetworkVideoTransmitter,
    }
    impl Default for ProbeType {
        fn default() -> ProbeType {
            Self::Device
        }
    }
    #[derive(Default, PartialEq, Debug, YaDeserialize, YaSerialize)]
    #[yaserde(
        prefix = "d",
        namespace = "d: http://schemas.xmlsoap.org/ws/2005/04/discovery",
        namespace = "wsdl: http://www.onvif.org/ver10/network/wsdl"
    )]
    pub struct Probe {
        #[yaserde(prefix = "d", rename = "Types")]
        pub probe_types: Vec<String>,
    }

    #[derive(Default, PartialEq, Debug, YaDeserialize, YaSerialize)]
    #[yaserde(
        prefix = "d",
        namespace = "d: http://schemas.xmlsoap.org/ws/2005/04/discovery",
        namespace = "wsa: http://schemas.xmlsoap.org/ws/2004/08/addressing"
    )]
    pub struct ProbeMatch {
        #[yaserde(prefix = "d", rename = "XAddrs")]
        pub xaddrs: String,
        #[yaserde(prefix = "wsa", rename = "EndpointReference")]
        pub endpoint_reference: String,
        #[yaserde(prefix = "d", rename = "Types")]
        pub types: Vec<ProbeType>,
        #[yaserde(prefix = "d", rename = "Scopes")]
        pub scopes: Vec<String>,
        #[yaserde(prefix = "d", rename = "MetadataVersion")]
        pub metadata_version: String,
    }

    #[derive(Default, PartialEq, Debug, YaDeserialize, YaSerialize)]
    #[yaserde(
        prefix = "d",
        namespace = "d: http://schemas.xmlsoap.org/ws/2005/04/discovery"
    )]
    pub struct ProbeMatches {
        #[yaserde(prefix = "d", rename = "ProbeMatch")]
        pub probe_match: Vec<ProbeMatch>,
    }
}

pub mod util {
    use super::{common, to_deserialize, to_serialize};
    use log::{error, info, trace};
    use std::{
        net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
        sync::{mpsc, Arc, Mutex},
        thread,
        time::Duration,
    };

    pub fn simple_onvif_discover(timeout_seconds: u64) -> Result<Vec<String>, failure::Error> {
        let (sender, receiver) = mpsc::channel();
        let shared_devices = Arc::new(Mutex::new(Vec::new()));

        const LOCAL_IPV4_ADDR: Ipv4Addr = Ipv4Addr::UNSPECIFIED;
        const LOCAL_PORT: u16 = 0;

        const MULTI_IPV4_ADDR: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);
        const MULTI_PORT: u16 = 3702;

        let local_socket_addr = SocketAddr::new(IpAddr::V4(LOCAL_IPV4_ADDR), LOCAL_PORT);
        let multi_socket_addr = SocketAddr::new(IpAddr::V4(MULTI_IPV4_ADDR), MULTI_PORT);

        trace!(
            "simple_onvif_discover ... in spawned thread ... binding to: {:?}",
            local_socket_addr
        );
        let socket = UdpSocket::bind(local_socket_addr).unwrap();
        let shared_socket = Arc::new(Mutex::new(socket));

        let thread_devices = shared_devices.clone();

        let (signal_request_sent, wait_for_signal) = mpsc::channel();
        let send_socket = shared_socket.clone();
        thread::spawn(move || {
            trace!("simple_onvif_discover ... in spawned send thread");

            trace!(
                "simple_onvif_discover ... in spawned thread ... joining multicast: {:?} {:?}",
                &MULTI_IPV4_ADDR,
                &LOCAL_IPV4_ADDR
            );
            send_socket
                .lock()
                .unwrap()
                .join_multicast_v4(&MULTI_IPV4_ADDR, &LOCAL_IPV4_ADDR)
                .unwrap();
            let probe_types: Vec<String> = vec!["wsdl:NetworkVideoTransmitter".into()];
            let envelope = to_serialize::Envelope {
                header: to_serialize::Header {
                    message_id: format!("uuid:{}", uuid::Uuid::new_v4()),
                    action: "http://schemas.xmlsoap.org/ws/2005/04/discovery/Probe".into(),
                    reply_to: "urn:schemas-xmlsoap-org:ws:2005:04:discovery".into(),
                    ..Default::default()
                },
                body: to_serialize::Body {
                    probe: common::Probe {
                        probe_types: probe_types,
                    },
                    ..Default::default()
                },
                ..Default::default()
            };
            let envelope_as_string = yaserde::ser::to_string(&envelope).unwrap();
            trace!("-------> serialized: [{}]", envelope_as_string);
            trace!(
                "simple_onvif_discover ... in spawned send thread ... sending: {:?}",
                &envelope_as_string
            );
            send_socket
                .lock()
                .unwrap()
                .send_to(&envelope_as_string.as_bytes(), multi_socket_addr)
                .unwrap();
            trace!("simple_onvif_discover ... in spawned send thread ... signalling finished");
            signal_request_sent.send(()).unwrap();
            trace!("simple_onvif_discover ... exit spawned send thread");
        });

        let listen_socket = shared_socket.clone();
        thread::spawn(move || {
            trace!("simple_onvif_discover ... in spawned listen thread");
            wait_for_signal.recv().unwrap();
            trace!("simple_onvif_discover ... in spawned listen thread ... start listening for responses");
            loop {
                let mut buf = vec![0; 16 * 1024];
                match listen_socket.lock().unwrap().recv_from(&mut buf) {
                    Ok((len, _src)) => {
                        trace!("simple_onvif_discover ... in spawned listen thread ... recv_from: {:?} {:?}", len, _src);
                        let broadcast_response_as_string =
                            String::from_utf8_lossy(&buf[..len]).to_string();
                        trace!(
                            "simple_onvif_discover ... in spawned listen thread ... response: {:?}",
                            broadcast_response_as_string
                        );
                        let response_envelope = yaserde::de::from_str::<to_deserialize::Envelope>(
                            &broadcast_response_as_string,
                        );
                        let _consume_the_iterators = response_envelope
                            .unwrap()
                            .body
                            .probe_matches
                            .probe_match
                            .iter()
                            .flat_map(|probe_match| probe_match.xaddrs.split_whitespace())
                            .map(|x| {
                                scan_fmt!(x, "http://{}/onvif/device_service", String).unwrap()
                            })
                            .map(|x| thread_devices.lock().unwrap().push(x.to_string()))
                            .count();
                    }
                    Err(_e) => {
                        error!("simple_onvif_discover ... in spawned listen thread ... recv_from: {:?}", _e);
                        break;
                    }
                }
            }
            trace!("simple_onvif_discover ... in spawned listen thread ... finished");
            sender.send(()).unwrap();
        });

        trace!(
            "simple_onvif_discover ... wait for thread to finish or {} seconds",
            timeout_seconds
        );
        let receiver_result = receiver.recv_timeout(Duration::from_secs(timeout_seconds));
        trace!(
            "simple_onvif_discover ... thread finished or timeout: {:?}",
            receiver_result
        );
        let result_devices = shared_devices.lock().unwrap().clone();
        info!("simple_onvif_discover ... devices: {:?}", result_devices);
        Ok(result_devices)
    }
}
