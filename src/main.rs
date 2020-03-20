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

use sxd_document;
use sxd_xpath;

use blake2::VarBlake2b;
use blake2::digest::{Input, VariableOutput};

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

pub const GET_NETWORK_INTERFACES_TEMPLATE: &str = r#"<soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope" xmlns:wsdl="http://www.onvif.org/ver10/device/wsdl">
   <soap:Header/>
   <soap:Body>
      <wsdl:GetNetworkInterfaces/>
   </soap:Body>
</soap:Envelope>"#;

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

pub const GET_HOSTNAME_TEMPLATE: &str = r#"<soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope" xmlns:wsdl="http://www.onvif.org/ver10/device/wsdl">
   <soap:Header/>
   <soap:Body>
      <wsdl:GetHostname/>
   </soap:Body>
</soap:Envelope>"#;


#[tokio::main]
async fn main() {
    env_logger::try_init().unwrap();

   async {
      trace!("enter my-onvif");

      let devices: Vec<String> =
         match std::env::var("DISCOVERED_DEVICES") {
            Ok(env_devices) => env_devices.split(";").map(|s| s.to_string()).collect(),
            _ => util::simple_onvif_discover(1).unwrap(),
      };
      info!("onvif devices found: {:?}", devices);
      for device in devices {

         info!("get service uris for: {:?}", device);
         let services_xml = simple_post(
            &"onvif/device_service".to_string(),
            &device,
            &r#"action="http://www.onvif.org/ver10/device/wsdl/GetServices""#.to_string(), 
            &GET_SERVICES_TEMPLATE.to_string()).unwrap();
         let services_doc = services_xml.as_document();
         let device_service_uri = sxd_xpath::evaluate_xpath(
               &services_doc,
               "//*[local-name()='GetServicesResponse']/*[local-name()='Service' and *[local-name()='Namespace']/text() ='http://www.onvif.org/ver10/device/wsdl']/*[local-name()='XAddr']/text()"
            )
            .unwrap()
            .string();
         info!("Device service uris: {:?}", device_service_uri);
         let media_service_uri = sxd_xpath::evaluate_xpath(
               &services_doc,
               "//*[local-name()='GetServicesResponse']/*[local-name()='Service' and *[local-name()='Namespace']/text() ='http://www.onvif.org/ver10/media/wsdl']/*[local-name()='XAddr']/text()"
            )
            .unwrap()
            .string();
         info!("Media service uris: {:?}", media_service_uri);
         
         info!("get device information for: {:?}", device);
            let device_information_xml = simple_post(
               &"onvif/device_service".to_string(), 
               &device, 
               &r#"action="http://www.onvif.org/ver10/device/wsdl/GetDeviceInformation""#.to_string(), 
               &GET_DEVICE_INFORMATION_TEMPLATE.to_string()).unwrap();
            let device_information_doc = device_information_xml.as_document();
            let serial_number = sxd_xpath::evaluate_xpath(
                  &device_information_doc,
                  "//*[local-name()='GetDeviceInformationResponse']/*[local-name()='SerialNumber']/text()"
               )
               .unwrap()
               .string();
            info!("Device information (serial number): {:?}", serial_number);

            let network_interfaces_xml = simple_post(
               &"onvif/device_service".to_string(),
               &device,
               &r#"action="http://www.onvif.org/ver10/device/wsdl/GetNetworkInterfaces""#.to_string(), 
               &GET_NETWORK_INTERFACES_TEMPLATE.to_string()).unwrap();
            let network_interfaces_doc = network_interfaces_xml.as_document();
            let mac_address = sxd_xpath::evaluate_xpath(
                  &network_interfaces_doc,
                  "//*[local-name()='GetNetworkInterfacesResponse']/*[local-name()='NetworkInterfaces']/*[local-name()='Info']/*[local-name()='HwAddress']/text()"
               )
               .unwrap()
               .string();
            info!("Network interfaces (mac address): {:?}", mac_address);

            let id = format!("{}-{}", device, mac_address);
            let mut hasher = VarBlake2b::new(3).unwrap();
            hasher.input(id.clone());
            let digest = hasher
               .vec_result()
               .iter()
               .map(|num| format!("{:02X}", num))
               .collect::<Vec<String>>()
               .join("");
            info!("DIGEST {}: {}", id, digest);

            let hostname_xml = simple_post(
               &"onvif/device_service".to_string(),
               &device,
               &r#"action="http://www.onvif.org/ver10/device/wsdl/GetHostname""#.to_string(), 
               &GET_HOSTNAME_TEMPLATE.to_string()).unwrap();
            let hostname_doc = hostname_xml.as_document();
            let hostname = sxd_xpath::evaluate_xpath(
                  &hostname_doc,
                  "//*[local-name()='GetHostnameResponse']/*[local-name()='HostnameInformation']/*[local-name()='Name']/text()"
               )
               .unwrap()
               .string();
            info!("Hostname: {:?}", hostname);
            
            info!("get scopes for: {:?}", device);
            let scopes_xml = simple_post(
               &"onvif/device_service".to_string(), 
               &device, 
               &r#"action="http://www.onvif.org/ver10/device/wsdl/GetScopes""#.to_string(), 
               &GET_SCOPES_TEMPLATE.to_string()).unwrap();
            let scopes_doc = scopes_xml.as_document();
            let scopes_query = sxd_xpath::evaluate_xpath(
               &scopes_doc,
               "//*[local-name()='GetScopesResponse']/*[local-name()='Scopes']/*[local-name()='ScopeItem']/text()"
            );
            let scopes = match scopes_query {
                  Ok(sxd_xpath::Value::Nodeset(scope_items)) => {
                     scope_items.iter().map(|scope_item| scope_item.string_value() ).collect::<Vec<String>>()
                  },
                  _ => panic!("TODO: uses error not panic!")
               };
            info!("Scopes: {:?}", scopes);

            info!("get profiles and streaming uris for: {:?}", device);
            let profiles_xml = simple_post(
               &"onvif/Media".to_string(),
               &device,
               &r#"action="http://www.onvif.org/ver10/media/wsdl/GetProfiles""#.to_string(), 
               &GET_PROFILES_TEMPLATE.to_string()).unwrap();
            let profiles_doc = profiles_xml.as_document();
            let profiles_query = sxd_xpath::evaluate_xpath(
               &profiles_doc,
               "//*[local-name()='GetProfilesResponse']/*[local-name()='Profiles']/@token"
            );
            let uri_info = match profiles_query {
                  Ok(sxd_xpath::Value::Nodeset(tokens)) => {
                     tokens.iter().map(|profile_token| {
                        let stream_soap = get_stream_uri_message(&profile_token.string_value());
                        let stream_uri_xml = simple_post(
                           &"onvif/Media".to_string(),
                           &device,
                           &r#"action="http://www.onvif.org/ver10/media/wsdl/GetStreamUri""#.to_string(),
                           &stream_soap).unwrap();
                        let stream_uri_doc = stream_uri_xml.as_document();
                        let stream_uri = sxd_xpath::evaluate_xpath(
                              &stream_uri_doc,
                              "//*[local-name()='GetStreamUriResponse']/*[local-name()='MediaUri']/*[local-name()='Uri']/text()"
                           )
                           .unwrap()
                           .string();
                        (profile_token.string_value(), stream_uri)
                     }).collect::<Vec<(String, String)>>()
                  },
                  _ => panic!("TODO: replace iwth error")
               };
            info!("Profile tokens: {:?}", uri_info);
      }
      trace!("exit my-onvif");
    }.await;
}

fn simple_post(url: &String, ip: &String, mime_action: &String, msg: &String) -> Result<sxd_document::Package, failure::Error> {
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

   let post = client.request(req).and_then(|res| { 
      trace!("pre res.body.concat2 response: {:?}", res);
      res.body().concat2()
         .and_then(move |body| {
            let v = body.to_vec();
            let xml_as_string = String::from_utf8_lossy(&v);
            trace!("post res.body.concat2 Response as string: {:?}", xml_as_string);

            //let xml_as_tree = xmltree::Element::parse(xml_as_string.as_bytes()).unwrap();
            let xml_as_tree = sxd_document::parser::parse(&xml_as_string).unwrap();
            trace!("post res.body.concat2 Response as xmltree: {:?}", xml_as_tree);
            Ok(xml_as_tree)
         })
   });
   let dom = core.run(post).expect("failed to make request");
   Ok(dom)
}

