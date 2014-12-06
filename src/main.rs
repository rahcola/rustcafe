#![feature(slicing_syntax)]
extern crate serialize;
extern crate hyper;

use serialize::json;
use hyper::Url;
use hyper::status::StatusCode;
use hyper::client::Request;
use hyper::client::Response;

#[deriving(Decodable, Show)]
struct ApiResponse<T> {
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

fn api(url_str: &str) -> Response {
    let url = match Url::parse(url_str) {
        Ok(u) => u,
        Err(e) => panic!("bad url {}: {}", url_str, e),
    };
    let res = match Request::get(url)
        .and_then(|r| { r.start() })
        .and_then(|r| { r.send() }) {
            Ok(r @ Response {status: StatusCode::Ok, ..})
                => r,
            Ok(Response {status: x, ..})
                => panic!("GET {} failed: {}", url_str, x),
            Err(e)
                => panic!("GET {} failed: {}", url_str, e),
        };
    res
}

fn restaurants() -> Vec<Restaurant> {
    let url = "http://messi.hyyravintolat.fi/publicapi/restaurants";
    let res = api(url[]).read_to_string().unwrap();
    let response: ApiResponse<Vec<Restaurant>> = json::decode(res[]).unwrap();
    response.data
}

fn menus(id: u64) -> Vec<Menu> {
    let url = format!("http://messi.hyyravintolat.fi/publicapi/restaurant/{}", id);
    let res = api(url[]).read_to_string().unwrap();
    let response: ApiResponse<Vec<Menu>> = json::decode(res[]).unwrap();
    response.data
}

fn main() {
    println!("{}", menus(11));
}
