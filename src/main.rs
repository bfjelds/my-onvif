#[macro_use]
extern crate log;
#[macro_use]
extern crate yaserde_derive;
#[macro_use]
extern crate scan_fmt;

extern crate hyper;
extern crate tokio_core;

use env_logger;

use hyper::{
   Client,
   Uri,
};

use futures::{Future, Stream};
use tokio_core::reactor::Core;

mod onvif;
use onvif::util;


pub const GET_STREAM_URI_TEMPLATE: &str = r#"<?xml version='1.0' encoding='utf-8'?>
<soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope" xmlns:wsdl="http://www.onvif.org/ver10/media/wsdl" xmlns:sch="http://www.onvif.org/ver10/schema">
   <soap:Header/>
   <soap:Body>
      <wsdl:GetStreamUri>
         <wsdl:StreamSetup>
            <sch:Stream>RTP-Unicast</sch:Stream>
            <sch:Transport>
               <sch:Protocol>RTSP</sch:Protocol>
            </sch:Transport>
         </wsdl:StreamSetup>
         <wsdl:ProfileToken>{profile_token}</wsdl:ProfileToken>
      </wsdl:GetStreamUri>
   </soap:Body>
</soap:Envelope>;"#;

pub const GET_PROFILES_TEMPLATE: &str = r#"<?xml version='1.0' encoding='utf-8'?>
<soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope" xmlns:wsdl="http://www.onvif.org/ver10/media/wsdl">
   <soap:Header/>
   <soap:Body>
      <wsdl:GetProfiles/>
   </soap:Body>
</soap:Envelope>"#;

#[tokio::main]
async fn main() {
    env_logger::try_init().unwrap();

   async {
      trace!("enter my-onvif");
      let devices = util::simple_onvif_discover().unwrap();
      info!("onvif devices found: {:?}", devices);
      for device in devices {
            info!("get profiles for: {:?}", device);
            let profiles = simple_get_onvif_profiles(&device);
            info!("get stream for: {:?}", device);
      }
      trace!("exit my-onvif");
    }.await;
}

fn simple_get_onvif_profiles(device: &String) -> Result<Vec<String>, failure::Error> {
   let mut profiles: Vec<String> = Vec::new();
   let mut core = Core::new().unwrap();
   let handle = core.handle();
   let client = Client::new(&handle);

   let uri: Uri = format!("http://{}/onvif/Media", device).parse().unwrap();

   // let req = hyper::Request::builder()
   //    .method(hyper::Method::POST)
   //    .header(hyper::header::CONTENT_TYPE, "application/soap+xml")
   //    .body(hyper::Body::from(GET_PROFILES_TEMPLATE));

    let mut req = hyper::Request::new(hyper::Method::Post, uri);
    let content_type: hyper::mime::Mime = "application/soap+xml".parse().unwrap();
    req.headers_mut().set(hyper::header::ContentType(content_type));
    req.set_body(hyper::Body::from(GET_PROFILES_TEMPLATE));

   //  let req = hyper::Request::builder()
   //       .method(hyper::Method::POST)
   //       .uri(uri)
   //       .header(hyper::header::CONTENT_TYPE, "application/soap+xml")
   //       .body(hyper::Body::from(GET_PROFILES_TEMPLATE))
   //       .expect("request builder");

   let post = client.request(req).and_then(|res| { 
      trace!("pre res.body.concat2 response: {:?}", res);
      res.body().concat2()
         .and_then(move |body| {
            let v = body.to_vec();
            trace!("post res.body.concat2 Response: {:?}", String::from_utf8_lossy(&v));
            Ok(body)
         })
   });
   let request_result = core.run(post);
   Ok(profiles)
}