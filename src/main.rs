#![feature(plugin)]
#![plugin(regex_macros)]
extern crate core;
extern crate chrono;
extern crate hyper;
extern crate regex;
extern crate regex_macros;
extern crate "rustc-serialize" as rustc_serialize;
extern crate docopt;

use chrono::{Date, UTC, FixedOffset, Datelike, Weekday};
use docopt::Docopt;
use hyper::Client;
use hyper::status::StatusCode;
use rustc_serialize::{json, Decoder, Decodable};
use std::error::{FromError, Error};
use std::io::Read;
use std::{io, old_io, fmt};
use std::num::ParseIntError;
use std::str::{FromStr};

#[derive(RustcDecodable, Debug)]
struct ApiResponse<T> {
    status: String,
    data: T,
}

#[derive(RustcDecodable, Debug)]
struct Restaurant {
    id: u64,
    name: String,
}

#[derive(PartialEq, Eq, Debug)]
struct UnicafeDate(Date<FixedOffset>);

#[derive(Debug)]
enum PriceClass {
    Bistro,
    Maukkaasti,
    Edullisesti,
    Keitto,
    Kevyesti,
    Makeasti,
}

#[derive(RustcDecodable, Debug)]
struct Price {
    name: PriceClass,
}

#[derive(RustcDecodable, Debug)]
struct Food {
    name: String,
    price: Price,
}

#[derive(RustcDecodable, Debug)]
struct Menu {
    date: UnicafeDate,
    data: Vec<Food>,
}

impl fmt::Display for UnicafeDate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
        let day_s = try!(cap.at(1).ok_or(d.error("no day given")));
        let day = try!(FromStr::from_str(day_s)
                       .map_err(|e: ParseIntError| d.error(e.description())));
        let mon_s = try!(cap.at(2).ok_or(d.error("no month given")));
        let mon = try!(FromStr::from_str(mon_s)
                       .map_err(|e: ParseIntError| d.error(e.description())));
        Ok(UnicafeDate(try!(unicafe_today()
                            .with_month(mon)
                            .ok_or(d.error("invalid month"))
                            .and_then(|now| now.with_day(day)
                                      .ok_or(d.error("invalid day"))))))
    }
}

impl fmt::Display for PriceClass {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

#[derive(Debug)]
enum UnicafeError {
    BadStatusCode(StatusCode),
    DecoderError(json::DecoderError),
    HttpError(hyper::HttpError),
    IoError(io::Error),
    OldIoError(old_io::IoError),
    NoFoodToday,
    NoSuchRestaurant(String),
}

impl Error for UnicafeError {
    fn description(&self) -> &str {
        match *self {
            UnicafeError::BadStatusCode(..) => "bad HTTP status code",
            UnicafeError::DecoderError(ref e) => e.description().clone(),
            UnicafeError::HttpError(ref e) => e.description().clone(),
            UnicafeError::IoError(ref e) => e.description().clone(),
            UnicafeError::OldIoError(ref e) => e.description().clone(),
            UnicafeError::NoFoodToday => "no food today",
            UnicafeError::NoSuchRestaurant(..) => "no such restaurant",
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            UnicafeError::BadStatusCode(_) => None,
            UnicafeError::DecoderError(ref e) => Some(e),
            UnicafeError::HttpError(ref e) => Some(e),
            UnicafeError::IoError(ref e) => Some(e),
            UnicafeError::OldIoError(ref e) => Some(e),
            UnicafeError::NoFoodToday => None,
            UnicafeError::NoSuchRestaurant(_) => None,
        }
    }
}

impl fmt::Display for UnicafeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            UnicafeError::BadStatusCode(ref code)
                => write!(f, "bad status code: {}", code),
            UnicafeError::NoSuchRestaurant(ref restaurant)
                => write!(f, "no restaurant {}", restaurant),
            ref e => write!(f, "{:?}", e),
        }
    }
}

impl FromError<hyper::HttpError> for UnicafeError {
    fn from_error(err: hyper::HttpError) -> UnicafeError {
        UnicafeError::HttpError(err)
    }
}

impl FromError<old_io::IoError> for UnicafeError {
    fn from_error(err: old_io::IoError) -> UnicafeError {
        UnicafeError::OldIoError(err)
    }
}

impl FromError<io::Error> for UnicafeError {
    fn from_error(err: io::Error) -> UnicafeError {
        UnicafeError::IoError(err)
    }
}

impl FromError<rustc_serialize::json::DecoderError> for UnicafeError {
    fn from_error(err: rustc_serialize::json::DecoderError) -> UnicafeError {
        UnicafeError::DecoderError(err)
    }
}

fn unicafe_today() -> Date<FixedOffset> {
    UTC::today().with_timezone(&FixedOffset::east(60*60*2))
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
    let mut json_str = String::new();
    match res {
        hyper::client::Response {status: StatusCode::Ok, ..}
          => try!(res.read_to_string(&mut json_str)),
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

#[derive(RustcDecodable, Debug)]
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
    let args: Args = Docopt::new(USAGE).and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());
    doit(args).unwrap_or_else(|e| println!("{:?}", e));
}
