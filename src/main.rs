extern crate clap;
extern crate jxr;
use clap::{App, Arg};
use jxr::Items;
use std::path::PathBuf;

fn main() {
    let matches = App::new("jxr")
        .version("1.0.0")
        .author("Joe Moon <joe@xoxomoon.com>")
        .about("Generate a static site.")
        .arg(
            Arg::with_name("path")
                .help("Sets the input file to use")
                .index(1)
                .default_value("."),
        )
        .arg(
            Arg::with_name("verbose")
                .long("verbose")
                .short("v")
                .help("Verbose mode"),
        )
        .arg(
            Arg::with_name("output_path")
                .long("output")
                .short("o")
                .default_value(".")
                .help("output directory"),
        )
        .get_matches();

    // let verbose = matches.is_present("verbose");
    let root_path = PathBuf::from(matches.value_of("path").unwrap());
    let output_path = PathBuf::from(matches.value_of("output_path").unwrap());

    let items = Items::new(&root_path).expect("Error getting items");
    for item in items.items.into_iter() {
        item.write(&output_path).unwrap();
    }
}
