#![feature(plugin)]
extern crate core;
extern crate chrono;
extern crate hyper;
extern crate regex;
#[plugin] extern crate regex_macros;
extern crate "rustc-serialize" as rustc_serialize;
extern crate docopt;
extern crate url;

use core::fmt;
use chrono::{Date, UTC, FixedOffset, Datelike, Weekday};
use docopt::Docopt;
use hyper::Client;
use hyper::status::StatusCode;
use rustc_serialize::{json, Decoder, Decodable};
use std::error::{FromError, Error};
use std::io;
use std::str::{FromStr};
use url::ParseError;

#[derive(RustcDecodable, Show)]
struct ApiResponse<T> {
    status: String,
    data: T,
}

#[derive(RustcDecodable, Show)]
struct Restaurant {
    id: u64,
    name: String,
}

#[derive(PartialEq, Eq, Show)]
struct UnicafeDate(Date<FixedOffset>);

#[derive(Show)]
enum PriceClass {
    Bistro,
    Maukkaasti,
    Edullisesti,
    Keitto,
    Kevyesti,
    Makeasti,
}

#[derive(RustcDecodable, Show)]
struct Price {
    name: PriceClass,
}

#[derive(RustcDecodable, Show)]
struct Food {
    name: String,
    price: Price,
}

#[derive(RustcDecodable, Show)]
struct Menu {
    date: UnicafeDate,
    data: Vec<Food>,
}

impl fmt::String for UnicafeDate {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let UnicafeDate(ref d) = *self;
        write!(f, "{} {}.{}",
               finnish_weekday(d.weekday()), d.day(), d.month())
    }
}

impl Decodable for UnicafeDate {
    fn decode<D: Decoder>(d: &mut D) -> Result<UnicafeDate, D::Error> {
        let s = try!(d.read_str());
        let cap = try!(regex!(r"^[:alpha:]+ (\d{1,2})\.(\d{1,2})$")
                       .captures(&*s).ok_or(d.error("no date found")));
        let day = try!(cap.at(1).and_then(FromStr::from_str)
                       .ok_or(d.error("no day given")));
        let mon = try!(cap.at(2).and_then(FromStr::from_str)
                       .ok_or(d.error("no month given")));
        Ok(UnicafeDate(try!(unicafe_today()
                            .with_month(mon)
                            .ok_or(d.error("invalid month"))
                            .and_then(|now| now.with_day(day)
                                      .ok_or(d.error("invalid day"))))))
    }
}

impl fmt::String for PriceClass {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{:?}", self)
    }
}

impl Decodable for PriceClass {
    fn decode<D: Decoder>(d: &mut D) -> Result<PriceClass, D::Error> {
        match &*try!(d.read_str()) {
            "Bistro" => Ok(PriceClass::Bistro),
            "Maukkaasti" => Ok(PriceClass::Maukkaasti),
            "Edullisesti" => Ok(PriceClass::Edullisesti),
            "Keitto" => Ok(PriceClass::Keitto),
            "Kevyesti" => Ok(PriceClass::Kevyesti),
            "Makeasti" => Ok(PriceClass::Makeasti),
            x => Err(d.error(&*format!("unknown price {}", x))),
        }
    }
}

#[derive(Show)]
enum UnicafeError {
    BadStatusCode(StatusCode),
    DecoderError(json::DecoderError),
    HttpError(hyper::HttpError),
    IoError(io::IoError),
    NoFoodToday,
    NoSuchRestaurant(String),
    ParseError(url::ParseError),
}

impl Error for UnicafeError {
    fn description(&self) -> &str {
        match *self {
            UnicafeError::BadStatusCode(..) => "bad HTTP status code",
            UnicafeError::DecoderError(ref e) => e.description().clone(),
            UnicafeError::HttpError(ref e) => e.description().clone(),
            UnicafeError::IoError(ref e) => e.description().clone(),
            UnicafeError::NoFoodToday => "no food today",
            UnicafeError::NoSuchRestaurant(..) => "no such restaurant",
            UnicafeError::ParseError(ref e) => e.description().clone(),
        }
    }

    fn detail(&self) -> Option<String> {
        match *self {
            UnicafeError::BadStatusCode(ref code)
                => Some(format!("{}", code)),
            UnicafeError::NoSuchRestaurant(ref restaurant)
                => Some(restaurant.clone()),
            UnicafeError::DecoderError(ref e)
                => e.detail(),
            _ => None,
        }
    }
}

impl FromError<hyper::HttpError> for UnicafeError {
    fn from_error(err: hyper::HttpError) -> UnicafeError {
        UnicafeError::HttpError(err)
    }
}

impl FromError<io::IoError> for UnicafeError {
    fn from_error(err: io::IoError) -> UnicafeError {
        UnicafeError::IoError(err)
    }
}

impl FromError<rustc_serialize::json::DecoderError> for UnicafeError {
    fn from_error(err: rustc_serialize::json::DecoderError) -> UnicafeError {
        UnicafeError::DecoderError(err)
    }
}

impl FromError<url::ParseError> for UnicafeError {
    fn from_error(err: url::ParseError) -> UnicafeError {
        UnicafeError::ParseError(err)
    }
}

fn unicafe_today() -> Date<FixedOffset> {
    UTC::today().with_offset(FixedOffset::east(60*60*2))
}

fn finnish_weekday(w: Weekday) -> &'static str {
    use chrono::Weekday::*;
    match w {
        Mon => "Ma",
        Tue => "Ti",
        Wed => "Ke",
        Thu => "To",
        Fri => "Pe",
        Sat => "La",
        Sun => "Su",
    }
}

fn restaurant_id(rs: &Vec<Restaurant>, name: &str) -> Option<u64> {
    rs.iter().find(|r| &*r.name == name).map(|r| r.id)
}

fn todays_menu(menus: &Vec<Menu>) -> Option<&Menu> {
    let today = UnicafeDate(unicafe_today());
    menus.iter().find(|m| m.date == today && m.data.len() > 0)
}

fn api<T: Decodable>(url: &str) -> Result<T, UnicafeError> {
    let mut client = Client::new();
    let mut res = try!(client.get(url).send());
    let json_str = match res {
        hyper::client::Response {status: StatusCode::Ok, ..}
          => try!(res.read_to_string()),
        hyper::client::Response {status: x, ..}
          => return Err(UnicafeError::BadStatusCode(x)),
    };
    Ok((try!(json::decode::<ApiResponse<T>>(&*json_str))).data)
}

fn restaurants() -> Result<Vec<Restaurant>, UnicafeError> {
    api("http://messi.hyyravintolat.fi/publicapi/restaurants")
}

fn menus(id: u64) -> Result<Vec<Menu>, UnicafeError> {
    api(&*format!("http://messi.hyyravintolat.fi/publicapi/restaurant/{}", id))
}

static USAGE: &'static str = "
Usage: rustcafe [--today] <restaurant>

Options:
    --today  display only today's menu
";

#[derive(RustcDecodable, Show)]
struct Args {
    arg_restaurant: String,
    flag_today: bool,
}

fn doit(args: Args) -> Result<(), UnicafeError> {
    let rs = try!(restaurants());
    let r = args.arg_restaurant;
    let id = try!(restaurant_id(&rs, &*r)
                  .ok_or(UnicafeError::NoSuchRestaurant(r)));
    let menus = try!(menus(id));
    if args.flag_today {
        let menu = try!(todays_menu(&menus).ok_or(UnicafeError::NoFoodToday));
        for f in menu.data.iter() {
            println!("{}\t{}", f.price.name, f.name);
        }
    } else {
        for m in menus.iter() {
            println!("{}", m.date);
            for f in m.data.iter() {
                println!("\t{}\t{}", f.price.name, f.name);
            }
        }
    };
    Ok(())
}

fn main() {
    let args: Args = Docopt::new(USAGE).and_then(|d| d.decode()).unwrap();
    doit(args).unwrap_or_else(|e| println!("{:?}", e));
}
