#![feature(slicing_syntax)]
#![feature(macro_rules)]
extern crate serialize;
extern crate hyper;

use serialize::json;
use serialize::json::Json;
use hyper::Url;
use hyper::client::Request;

macro_rules! err_try(
    ($var:ident | $pat:pat <- $expr:expr) => {
        match $expr {
            $pat => $var,
            _ => panic!("err_try! failed."),
        }
    }
)

macro_rules! err_let(
    ($var:ident | $pat:pat <- $expr:expr) => {
        let $var = err_try!($var | $pat <- $expr)
    }
)

fn restaurants() -> Vec<(u64, String)> {
    let url = Url::parse("http://messi.hyyravintolat.fi/publicapi/restaurants/");
    let req = Request::get(url.unwrap()).unwrap();
    let response = req.start().unwrap().send().unwrap().read_to_string().unwrap();
    let json_value = json::from_str(response.as_slice());

    err_let!(o | Ok(Json::Object(o)) <- json_value);
    err_let!(restaurants | Some(&Json::Array(ref restaurants)) <- o.get("data"));

    let mut v = Vec::new();
    for o in restaurants.iter() {
        err_let!(r | &Json::Object(ref r) <- o);
        err_let!(id | Some(&Json::U64(id)) <- r.get("id"));
        err_let!(name | Some(&Json::String(ref name)) <- r.get("name"));
        v.push((id, name.to_string()));
    };
    v
}

fn menus(id: u64) -> Vec<(String, Vec<String>)> {
    let url = Url::parse(format!("http://messi.hyyravintolat.fi/publicapi/restaurant/{}", id)[]);
    let req = Request::get(url.unwrap()).unwrap();
    let response = req.start().unwrap().send().unwrap().read_to_string().unwrap();
    err_let!(o | Ok(Json::Object(o)) <- json::from_str(response.as_slice()));

    err_let!(menus | Some(&Json::Array(ref menus)) <- o.get("data"));
    menus.iter().map(|menu| {
        err_let!(menu_obj | &Json::Object(ref menu_obj) <- menu);
        err_let!(date | Some(&Json::String(ref date)) <- menu_obj.get("date_en"));
        let foods = err_try!(f | Some(&Json::Array(ref f)) <- menu_obj.get("data")).iter().map(|o| {
                err_let!(food_obj | &Json::Object(ref food_obj) <- o);
                (err_try!(name | Some(&Json::String(ref name)) <- food_obj.get("name"))).to_string()
        }).collect::<Vec<String>>();
        (date.to_string(), foods)
    }).collect::<Vec<(String, Vec<String>)>>()
}

fn main() {
    println!("{}", restaurants());
    println!("{}", menus(1));
}

