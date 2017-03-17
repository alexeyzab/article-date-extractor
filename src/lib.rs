#![recursion_limit = "1024"]
extern crate regex;
#[macro_use] extern crate lazy_static;
extern crate chrono;
extern crate reqwest;
extern crate select;
extern crate rustc_serialize;
#[macro_use] extern crate error_chain;
pub mod extract_date;
mod errors;
