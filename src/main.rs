extern crate serialize;
extern crate hyper;

use serialize::json;
use hyper::Url;
use hyper::client::Request;

fn main() {
    let url = Url::parse("http://messi.hyyravintolat.fi/publicapi/restaurants/");
    let req = Request::get(url.unwrap()).unwrap();
    let response = req.start().unwrap().send().unwrap().read_to_string().unwrap();
    let o = json::from_str(response.as_slice()).unwrap();
    let restaurants = o.as_object().unwrap().get("data").unwrap().as_array().unwrap();
    for o in restaurants.iter() {
        let name = o.as_object().unwrap().get("name").unwrap().as_string().unwrap();
        println!("{}", name);
    };
}

