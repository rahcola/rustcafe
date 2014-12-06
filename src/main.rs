#![feature(slicing_syntax, phase)]
extern crate docopt;
#[phase(plugin)]extern crate docopt_macros;
extern crate hyper;
extern crate serialize;

use docopt::Docopt;
use hyper::client::Request;
use hyper::client::Response;
use hyper::status::StatusCode;
use hyper::Url;
use serialize::json;

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

fn api<T: serialize::Decodable<serialize::json::Decoder,
                               serialize::json::DecoderError>>
    (url_str: &str) -> T {
    let url = Url::parse(url_str).unwrap();
    let res = match Request::get(url)
        .and_then(|r| r.start())
        .and_then(|r| r.send()) {
            Ok(ref mut r @ Response {status: StatusCode::Ok, ..})
                => r.read_to_string().unwrap(),
            Ok(Response {status: x, ..})
                => panic!("GET {} failed: {}", url_str, x),
            Err(e)
                => panic!("GET {} failed: {}", url_str, e),
        };
    let r: ApiResponse<T> = json::decode(res[]).unwrap();
    r.data
}

fn restaurants() -> Vec<Restaurant> {
    api("http://messi.hyyravintolat.fi/publicapi/restaurants")
}

fn menus(id: u64) -> Vec<Menu> {
    api(format!("http://messi.hyyravintolat.fi/publicapi/restaurant/{}", id)[])
}

fn restaurant_id(rs: &Vec<Restaurant>, name: &str) -> Option<u64> {
    for x in rs.iter() {
        if x.name[] == name {
            return Some(x.id)
        }
    }
    None
}

docopt!(Args deriving Show, "
Usage: rustcafe <restaurant>
")

fn main() {
    let args: Args = Args::docopt().decode().unwrap_or_else(|e| e.exit());
    let rs = restaurants();
    let r = args.arg_restaurant[];
    match restaurant_id(&rs, r) {
        Some(id) => for m in menus(id).iter() {
            println!("{}", m.date);
            for f in m.data.iter() {
                println!("\t{}", f.name);
            }
        },
        None => {
            println!("no restaurant {} exists", r);
            for r in rs.iter() {
                println!("{}", r.name[]);
            }
        },
    }
}
