#![feature(slicing_syntax)]
extern crate serialize;
extern crate hyper;

use serialize::json;
use hyper::Url;
use hyper::client::Request;

#[deriving(Decodable, Show)]
struct Response<T> {
    status: String,
    data: T,
}

#[deriving(Decodable, Show)]
struct Restaurant {
    id: u64,
    name: String,
}

fn restaurants() -> Vec<Restaurant> {
    let url = Url::parse("http://messi.hyyravintolat.fi/publicapi/restaurants").unwrap();
    let res = Request::get(url)
        .and_then(|r| { r.start() })
        .and_then(|r| { r.send() })
        .unwrap()
        .read_to_string()
        .unwrap();
    let response: Response<Vec<Restaurant>> = json::decode(res.as_slice()).unwrap();
    response.data
}

fn menus(id: u64) -> Vec<(String, Vec<String>)> {
    let url = Url::parse(format!("http://messi.hyyravintolat.fi/publicapi/restaurant/{}", id)[]);
    let req = Request::get(url.unwrap()).unwrap();
    let response = req.start().unwrap().send().unwrap().read_to_string().unwrap();
    let o = json::from_str(response.as_slice()).unwrap();
    let menus = o.as_object().unwrap().get("data").unwrap().as_array().unwrap();
    menus.iter().map(|menu| {
        let date = menu.as_object().unwrap().get("date_en").unwrap().as_string().unwrap().to_string();
        let foods = menu.as_object().unwrap().get("data")
            .unwrap().as_array().unwrap().iter().map(|o| {
                o.as_object().unwrap().get("name").unwrap().as_string().unwrap().to_string()
            }).collect::<Vec<String>>();
        (date, foods)
    }).collect::<Vec<(String, Vec<String>)>>()
}

fn main() {
    println!("{}", restaurants());
}
