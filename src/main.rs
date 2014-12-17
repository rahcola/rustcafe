#![feature(slicing_syntax, phase, globs)]
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
use hyper::Client;
use hyper::status::StatusCode;
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

#[deriving(PartialEq)]
struct UnicafeDate(Date<FixedOffset>);

#[deriving(Show)]
enum PriceClass {
    Bistro,
    Maukkaasti,
    Edullisesti,
    Keitto,
    Kevyesti,
    Makeasti,
}

#[deriving(Decodable, Show)]
struct Price {
    name: PriceClass,
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
        let UnicafeDate(ref d) = *self;
        write!(f, "{} {}.{}",
               finnish_weekday(d.weekday()), d.day(), d.month())
    }
}

impl<D: Decoder<E>, E> Decodable<D, E> for UnicafeDate {
    fn decode(d: &mut D) -> Result<UnicafeDate, E> {
        let s = try!(d.read_str());
        let cap = try!(regex!(r"^[:alpha:]+ (\d{1,2})\.(\d{1,2})$")
                       .captures(s[]).ok_or(d.error("no date found")));
        let day = try!(from_str(cap.at(1)).ok_or(d.error("no day given")));
        let mon = try!(from_str(cap.at(2)).ok_or(d.error("no month given")));
        Ok(UnicafeDate(try!(unicafe_today()
                            .with_month(mon)
                            .ok_or(d.error("invalid month"))
                            .and_then(|now| now.with_day(day)
                                      .ok_or(d.error("invalid day"))))))
    }
}

impl<D: Decoder<E>, E> Decodable<D, E> for PriceClass {
    fn decode(d: &mut D) -> Result<PriceClass, E> {
        match try!(d.read_str())[] {
            "Bistro" => Ok(PriceClass::Bistro),
            "Maukkaasti" => Ok(PriceClass::Maukkaasti),
            "Edullisesti" => Ok(PriceClass::Edullisesti),
            "Keitto" => Ok(PriceClass::Keitto),
            "Kevyesti" => Ok(PriceClass::Kevyesti),
            "Makeasti" => Ok(PriceClass::Makeasti),
            x => Err(d.error(format!("unknown price {}", x)[])),
        }
    }
}

enum UnicafeError {
    BadStatusCode(StatusCode),
    DecoderError(json::DecoderError),
    HttpError(hyper::HttpError),
    IoError(io::IoError),
    NoFoodToday,
    NoSuchRestaurant(String),
    ParseError(url::ParseError),
}

impl fmt::Show for UnicafeError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self.detail() {
            Some(msg) => write!(fmt, "{}: {}", self.description(), msg),
            None => write!(fmt, "{}", self.description()),
        }
    }
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

impl FromError<serialize::json::DecoderError> for UnicafeError {
    fn from_error(err: serialize::json::DecoderError) -> UnicafeError {
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
    rs.iter().find(|r| r.name[] == name).map(|r| r.id)
}

fn todays_menu(menus: &Vec<Menu>) -> Option<&Menu> {
    let today = UnicafeDate(unicafe_today());
    menus.iter().find(|m| m.date == today && m.data.len() > 0)
}

fn api<T: Decodable<json::Decoder, json::DecoderError>>(url: &str) -> Result<T, UnicafeError> {
    let mut client = Client::new();
    let mut res = try!(client.get(url).send());
    let json_str = match res {
        hyper::client::Response {status: StatusCode::Ok, ..}
          => try!(res.read_to_string()),
        hyper::client::Response {status: x, ..}
          => return Err(UnicafeError::BadStatusCode(x)),
    };
    Ok((try!(json::decode::<ApiResponse<T>>(json_str[]))).data)
}

fn restaurants() -> Result<Vec<Restaurant>, UnicafeError> {
    api("http://messi.hyyravintolat.fi/publicapi/restaurants")
}

fn menus(id: u64) -> Result<Vec<Menu>, UnicafeError> {
    api(format!("http://messi.hyyravintolat.fi/publicapi/restaurant/{}", id)[])
}

docopt!(Args deriving Show, "
Usage: rustcafe [--today] <restaurant>

Options:
    --today  display only today's menu
")

fn doit(args: Args) -> Result<(), UnicafeError> {
    let rs = try!(restaurants());
    let r = args.arg_restaurant;
    let id = try!(restaurant_id(&rs, r[])
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
    let args: Args = Args::docopt().decode().unwrap_or_else(|e| e.exit());
    doit(args).unwrap_or_else(|e| println!("{} {}", e, e.detail()));
}
