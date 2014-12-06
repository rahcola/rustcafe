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

#[deriving(Decodable, Show)]
struct Food {
    name: String,
}

#[deriving(Decodable, Show)]
struct Menu {
    date: String,
    data: Vec<Food>,
}

fn restaurants() -> Vec<Restaurant> {
    let url = Url::parse("http://messi.hyyravintolat.fi/publicapi/restaurants").unwrap();
    let res = Request::get(url)
        .and_then(|r| { r.start() })
        .and_then(|r| { r.send() })
        .unwrap()
        .read_to_string()
        .unwrap();
    let response: Response<Vec<Restaurant>> = json::decode(res[]).unwrap();
    response.data
}

fn menus(id: u64) -> Vec<Menu> {
    let url = Url::parse(format!("http://messi.hyyravintolat.fi/publicapi/restaurant/{}", id)[]).unwrap();
    let res = Request::get(url)
        .and_then(|r| { r.start() })
        .and_then(|r| { r.send() })
        .unwrap()
        .read_to_string()
        .unwrap();
    match json::decode(res[]) {
        Ok(Response { data: d, .. }) => d,
        Err(e) => {println!("{}", res[]); panic!(e)},
    }
}

fn main() {
    println!("{}", menus(11));
}
