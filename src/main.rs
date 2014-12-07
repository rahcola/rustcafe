#![feature(slicing_syntax, phase)]
extern crate core;
extern crate chrono;
extern crate docopt;
#[phase(plugin)]extern crate docopt_macros;
extern crate hyper;
extern crate regex;
#[phase(plugin)]extern crate regex_macros;
extern crate serialize;
extern crate url;

use core::fmt;
use chrono::{Date, UTC, FixedOffset, Datelike, Weekday};
use docopt::Docopt;
use hyper::client::Request;
use hyper::client::Response;
use hyper::status::StatusCode;
use hyper::Url;
use serialize::{json, Decoder, Decodable};
use std::error::{FromError, Error};
use std::io;
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

#[deriving(PartialEq, Eq)]
struct UnicafeDate(Date<FixedOffset>);

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
    date: UnicafeDate,
    data: Vec<Food>,
}

impl fmt::Show for UnicafeDate {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let UnicafeDate(d) = *self;
        write!(f, "{} {}.{}", finnish_weekday(d.weekday()), d.day(), d.month())
    }
}

impl<D: Decoder<E>, E> Decodable<D, E> for UnicafeDate {
    fn decode(d: &mut D) -> Result<UnicafeDate, E> {
        let now = unicafe_today();
        let s = try!(d.read_str());
        let cap = regex!(r"^[:alpha:]+ (\d{1,2})\.(\d{1,2})$")
            .captures(s[]).unwrap();
        let now = now.with_month(from_str(cap.at(2)).unwrap()).unwrap()
            .with_day(from_str(cap.at(1)).unwrap()).unwrap();
        Ok(UnicafeDate(now))
    }
}

#[deriving(Show)]
struct UnicafeError {
    message: Option<String>,
}

impl Error for UnicafeError {
    fn description(&self) -> &str { "Unicafe error" }
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

fn unicafe_today() -> Date<FixedOffset> {
    UTC::today().with_offset(FixedOffset::east(60*60*2))
}

fn finnish_weekday(w: Weekday) -> &'static str {
    match w {
        Weekday::Mon => "Ma",
        Weekday::Tue => "Ti",
        Weekday::Wed => "Ke",
        Weekday::Thu => "To",
        Weekday::Fri => "Pe",
        Weekday::Sat => "La",
        Weekday::Sun => "Su",
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
    rs.iter().find(|r| r.name[] == name).map(|r| r.id)
}

fn price_symbol(food: &Food) -> &'static str {
    match food.price.name[] {
        "Bistro" => "€€€€",
        "Maukkaasti" => "€€€",
        "Edullisesti" => "€€",
        _ => "€",
    }
}

docopt!(Args deriving Show, "
Usage: rustcafe [--today] <restaurant>

Options:
    --today  display only today's menu
")

fn doit(args: Args) -> Result<(), UnicafeError> {
    let rs = try!(restaurants());
    let r = args.arg_restaurant[];
    match restaurant_id(&rs, r) {
        Some(id) => {
            let menus = try!(menus(id));
            if args.flag_today {
                let today = UnicafeDate(unicafe_today());
                menus.iter().find(|m| m.date == today).map(|m| {
                    for f in m.data.iter() {
                        println!("{}\t{}", price_symbol(f), f.name);
                    }
                });
            } else {
                for m in menus.iter() {
                    println!("{}", m.date);
                    for f in m.data.iter() {
                        println!("\t{}\t{}", price_symbol(f), f.name);
                    }
                }
            };
            Ok(())
        },
        None => Err(UnicafeError{ message: Some(format!("no restaurant {} exists", r)) }),
    }
}

fn main() {
    let args: Args = Args::docopt().decode().unwrap_or_else(|e| e.exit());
    doit(args).unwrap_or_else(|e| println!("Runtime error: {}", e));
}
