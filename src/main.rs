extern crate serialize;
extern crate hyper;

use serialize::json;
use hyper::Url;
use hyper::client::Request;

fn restaurants() -> Vec<(u64, String)> {
    let url = Url::parse("http://messi.hyyravintolat.fi/publicapi/restaurants/");
    let req = Request::get(url.unwrap()).unwrap();
    let response = req.start().unwrap().send().unwrap().read_to_string().unwrap();
    let o = json::from_str(response.as_slice()).unwrap();
    let restaurants = o.as_object().unwrap().get("data").unwrap().as_array().unwrap();
    let mut v = Vec::new();
    for o in restaurants.iter() {
        let r = o.as_object().unwrap();
        let id = r.get("id").unwrap().as_u64().unwrap();
        let name = r.get("name").unwrap().as_string().unwrap();
        v.push((id, name.to_string()));
    };
    v
}

fn main() {
    for x in restaurants().iter() {
        println!("{}", x);
    }
}

