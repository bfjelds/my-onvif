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


fn get_stream_uri_message(profile: &String) -> String {
   format!(
      r#"<soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope" xmlns:wsdl="http://www.onvif.org/ver10/media/wsdl" xmlns:sch="http://www.onvif.org/ver10/schema">
         <soap:Header/>
         <soap:Body>
            <wsdl:GetStreamUri>
               <wsdl:StreamSetup>
                  <sch:Stream>RTP-Unicast</sch:Stream>
                  <sch:Transport>
                     <sch:Protocol>RTSP</sch:Protocol>
                  </sch:Transport>
               </wsdl:StreamSetup>
               <wsdl:ProfileToken>{}</wsdl:ProfileToken>
            </wsdl:GetStreamUri>
         </soap:Body>
      </soap:Envelope>;"#,
      profile
   )
}

pub const GET_PROFILES_TEMPLATE: &str = r#"<soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope" xmlns:wsdl="http://www.onvif.org/ver10/media/wsdl">
   <soap:Header/>
   <soap:Body>
      <wsdl:GetProfiles/>
   </soap:Body>
</soap:Envelope>"#;

pub const GET_SERVICES_TEMPLATE: &str = r#"<soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope" xmlns:wsdl="http://www.onvif.org/ver10/device/wsdl">
   <soap:Header/>
   <soap:Body>
      <wsdl:GetServices />
   </soap:Body>
</soap:Envelope>"#;

pub const GET_DEVICE_INFORMATION_TEMPLATE: &str = r#"<soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope" xmlns:wsdl="http://www.onvif.org/ver10/device/wsdl">
   <soap:Header/>
   <soap:Body>
      <wsdl:GetDeviceInformation/>
   </soap:Body>
</soap:Envelope>"#;

pub const GET_SCOPES_TEMPLATE: &str = r#"<soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope" xmlns:wsdl="http://www.onvif.org/ver10/device/wsdl">
   <soap:Header/>
   <soap:Body>
      <wsdl:GetScopes/>
   </soap:Body>
</soap:Envelope>"#;

#[tokio::main]
async fn main() {
    env_logger::try_init().unwrap();

   async {
      trace!("enter my-onvif");
      let devices: Vec<String> = util::simple_onvif_discover(1).unwrap();
      info!("onvif devices found: {:?}", devices);
      for device in devices {
            info!("get device information for: {:?}", device);
            let device_information_xmltree = simple_post(
               &"onvif/device_service".to_string(), 
               &device, 
               &r#"action="http://www.onvif.org/ver10/device/wsdl/GetDeviceInformation""#.to_string(), 
               &GET_DEVICE_INFORMATION_TEMPLATE.to_string()).unwrap();
            let device_information = device_information_xmltree
               .get_child("Body").unwrap()
               .get_child("GetDeviceInformationResponse").unwrap();
            trace!("Device information: {:?}", device_information);
            info!("get scopes for: {:?}", device);
            let scopes_xmltree = simple_post(
               &"onvif/device_service".to_string(), 
               &device, 
               &r#"action="http://www.onvif.org/ver10/device/wsdl/GetScopes""#.to_string(), 
               &GET_SCOPES_TEMPLATE.to_string()).unwrap();
            let scopes = scopes_xmltree
               .get_child("Body").unwrap()
               .get_child("GetScopesResponse").unwrap();
            trace!("Scopes: {:?}", scopes);
            info!("get services for: {:?}", device);
            let services_xmltree = simple_post(
               &"onvif/device_service".to_string(),
               &device,
               &r#"action="http://www.onvif.org/ver10/device/wsdl/GetServices""#.to_string(), 
               &GET_SERVICES_TEMPLATE.to_string()).unwrap();
            let media_uris = services_xmltree
               .get_child("Body").unwrap()
               .get_child("GetServicesResponse").unwrap()
               .children
               .to_owned()
               .into_iter()
               .filter(|service| service.as_element().unwrap().get_child("Namespace").unwrap().get_text().unwrap() == "http://www.onvif.org/ver10/media/wsdl")
               .map(|service| service.as_element().unwrap().get_child("XAddr").unwrap().get_text().unwrap().to_string())
               .collect::<Vec<String>>();
            trace!("Media uri: {:?}", media_uris.first().unwrap());
            info!("get profiles and streaming uris for: {:?}", device);
            let profiles_xmltree = simple_post(
               &"onvif/Media".to_string(),
               &device,
               &r#"action="http://www.onvif.org/ver10/media/wsdl/GetProfiles""#.to_string(), 
               &GET_PROFILES_TEMPLATE.to_string()).unwrap();
            let profile_uris = profiles_xmltree
               .get_child("Body").unwrap()
               .get_child("GetProfilesResponse").unwrap()
               .children
               .to_owned()
               .into_iter()
               .map(|profile| { 
                  let profile_token = profile.as_element().unwrap().attributes["token"].clone();
                  let stream_soap = get_stream_uri_message(&profile_token);
                  let stream_uri_xmltree = simple_post(
                     &"onvif/Media".to_string(),
                     &device,
                     &r#"action="http://www.onvif.org/ver10/media/wsdl/GetStreamUri""#.to_string(),
                     &stream_soap).unwrap();
                  let stream_uris = stream_uri_xmltree
                     .get_child("Body").unwrap()
                     .get_child("GetStreamUriResponse").unwrap()
                     .children
                     .to_owned()
                     .into_iter()
                     .map(|media_uri| media_uri.as_element().unwrap().get_child("Uri").unwrap().get_text().unwrap().to_string())
                     .collect::<Vec<String>>();
                  (profile_token, stream_uris.first().unwrap().to_string())
               })
               .collect::<Vec<(String, String)>>();
            info!("Profile token: {:?}", profile_uris);
            info!("get media stream uri for: {:?}", device);
      }
      trace!("exit my-onvif");
    }.await;
}

fn simple_post(url: &String, ip: &String, mime_action: &String, msg: &String) -> Result<xmltree::Element, failure::Error> {
   //let shared_profiles = Arc::new(Mutex::new(Vec::new()));
   //let mut profiles: Vec<xmltree::Element> = Vec::new();
   let mut core = Core::new().unwrap();
   let handle = core.handle();

   let client = Client::new(&handle);
   let uri: Uri = format!("http://{}:8899/{}", ip, url).parse().unwrap();
   let mut req = hyper::Request::new(hyper::Method::Post, uri);
   let body = hyper::Body::from(msg.clone().into_bytes());
   req.set_body(body);
   let full_mime = format!("{}; {}; {};", "application/soap+xml", "charset=utf-8", mime_action);
   let content_type: hyper::mime::Mime = full_mime.parse().unwrap();
   req.headers_mut().set(hyper::header::ContentType(content_type));
   req.headers_mut().set(hyper::header::ContentLength(msg.len() as u64));
   req.headers_mut().set(hyper::header::AcceptEncoding(vec![
      hyper::header::qitem(hyper::header::Encoding::Deflate), 
      hyper::header::qitem(hyper::header::Encoding::Gzip)
      ]));
   req.headers_mut().set(hyper::header::Connection::close());

   //let request_profiles = shared_profiles.clone();
   let post = client.request(req).and_then(|res| { 
      trace!("pre res.body.concat2 response: {:?}", res);
      res.body().concat2()
         .and_then(move |body| {
            let v = body.to_vec();
            let xml_as_string = String::from_utf8_lossy(&v);
            trace!("post res.body.concat2 Response as string: {:?}", xml_as_string);
            let xml_as_tree = xmltree::Element::parse(xml_as_string.as_bytes()).unwrap();
            trace!("post res.body.concat2 Response as xmltree: {:?}", xml_as_tree);
            //request_profiles.lock().unwrap().push(xml_as_tree.clone());
            Ok(xml_as_tree)
         })
   });
   let request_result = core.run(post).expect("failed to make request");
   Ok(request_result.clone())
}

