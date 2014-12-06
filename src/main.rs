#![feature(slicing_syntax, phase)]
extern crate docopt;
#[phase(plugin)]extern crate docopt_macros;
extern crate hyper;
extern crate serialize;
extern crate url;

use docopt::Docopt;
use hyper::client::Request;
use hyper::client::Response;
use hyper::status::StatusCode;
use hyper::Url;
use serialize::json;
use std::io;
use std::error::{FromError, Error};
use url::ParseError;

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
struct Price {
    name: String,
}

#[deriving(Decodable, Show)]
struct Food {
    name: String,
    price: Price,
}

#[deriving(Decodable, Show)]
struct Menu {
    date: String,
    data: Vec<Food>,
}

#[deriving(Show)]
struct UnicafeError {
    message: Option<String>,
}

impl Error for UnicafeError {
    fn description(&self) -> &str { "Unicafe error" }
    // XXX: is clone() really needed here?
    fn detail(&self) -> Option<String> { self.message.clone() }
}

impl FromError<io::IoError> for UnicafeError {
    fn from_error(err: io::IoError) -> UnicafeError {
        UnicafeError {
            message: err.detail(),
        }
    }
}

impl FromError<serialize::json::DecoderError> for UnicafeError {
    fn from_error(err: serialize::json::DecoderError) -> UnicafeError {
        UnicafeError {
            message: err.detail(),
        }
    }
}

impl FromError<url::ParseError> for UnicafeError {
    fn from_error(err: url::ParseError) -> UnicafeError {
        UnicafeError {
            message: err.detail(),
        }
    }
}

fn api<T: serialize::Decodable<serialize::json::Decoder,
                               serialize::json::DecoderError>>
    (url_str: &str) -> Result<T, UnicafeError> {
    let url = try!(Url::parse(url_str));
    let res = match Request::get(url)
        .and_then(|r| r.start())
        .and_then(|r| r.send()) {
            Ok(ref mut r @ Response {status: StatusCode::Ok, ..})
                => try!(r.read_to_string()),
            Ok(Response {status: x, ..})
                => return Err(UnicafeError{ message: Some(format!("GET {} failed: {}", url_str, x)), }),
            Err(e)
                => return Err(UnicafeError{ message: Some(format!("GET {} failed: {}", url_str, e)), }),
        };
    Ok((try!(json::decode::<ApiResponse<T>>(res[]))).data)
}

fn restaurants() -> Result<Vec<Restaurant>, UnicafeError> {
    api("http://messi.hyyravintolat.fi/publicapi/restaurants")
}

fn menus(id: u64) -> Result<Vec<Menu>, UnicafeError> {
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

fn price_symbol(food: &Food) -> &'static str {
    if food.price.name[] == "Bistro" {
        "€€€€"
    } else if food.price.name[] == "Maukkaasti" {
        "€€€"
    } else if food.price.name[] == "Edullisesti" {
        "€€"
    } else {
        "€"
    }
}

docopt!(Args deriving Show, "
Usage: rustcafe <restaurant>
")

fn doit(args: Args) -> Result<(), UnicafeError> {
    let rs = try!(restaurants());
    let r = args.arg_restaurant[];
    Ok(match restaurant_id(&rs, r) {
        Some(id) => for m in try!(menus(id)).iter() {
            println!("{}", m.date);
            for f in m.data.iter() {
                println!("\t{}\t{}", price_symbol(f), f.name);
            }
        },
        None => {
            println!("no restaurant {} exists", r);
            for r in rs.iter() {
                println!("{}", r.name[]);
            }
        },
    })
}

fn main() {
    let args: Args = Args::docopt().decode().unwrap_or_else(|e| e.exit());
    doit(args).unwrap_or_else(|e| println!("Runtime error: {}", e));
}
