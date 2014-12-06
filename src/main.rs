#![feature(slicing_syntax, phase)]
extern crate core;
extern crate chrono;
extern crate docopt;
#[phase(plugin)]extern crate docopt_macros;
extern crate hyper;
extern crate regex;
#[phase(plugin)]extern crate regex_macros;
extern crate serialize;

use core::fmt::{Error, Formatter, Show};
use chrono::{Date, UTC, FixedOffset, Datelike, Weekday};
use docopt::Docopt;
use hyper::client::Request;
use hyper::client::Response;
use hyper::status::StatusCode;
use hyper::Url;
use serialize::{json, Decoder, Decodable};

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

impl Show for UnicafeDate {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
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

fn unicafe_today() -> Date<FixedOffset> {
    UTC::today().with_offset(FixedOffset::east(60*60*2))
}

fn api<T: serialize::Decodable<json::Decoder, json::DecoderError>>
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

fn main() {
    let args: Args = Args::docopt().decode().unwrap_or_else(|e| e.exit());
    let rs = restaurants();
    let r = args.arg_restaurant[];
    match restaurant_id(&rs, r) {
        Some(id) => for m in menus(id).iter() {
            if args.flag_today {
                let UnicafeDate(date) = m.date;
                if date == unicafe_today() {
                    for f in m.data.iter() {
                        println!("{}\t{}", price_symbol(f), f.name);
                    }
                    return;
                }
            } else {
                println!("{}", m.date);
                for f in m.data.iter() {
                    println!("\t{}\t{}", price_symbol(f), f.name);
                }
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
