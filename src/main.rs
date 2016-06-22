#![cfg_attr(feature = "exotic_hash", feature(alloc))]
#![cfg_attr(feature = "exotic_hash", feature(core_intrinsics))]
#![cfg_attr(feature = "exotic_hash", feature(dropck_parametricity))]
#![cfg_attr(feature = "exotic_hash", feature(heap_api))]
#![cfg_attr(feature = "exotic_hash", feature(unique))]
#![cfg_attr(feature = "exotic_hash", feature(oom))]
#![cfg_attr(feature = "exotic_hash", feature(filling_drop))]
#![cfg_attr(feature = "exotic_hash", feature(unsafe_no_drop_flag))]

#[cfg(feature = "exotic_hash")]
extern crate alloc;
#[macro_use]
extern crate clap;
extern crate filetime;
extern crate fnv;

mod bit_set;
mod database;
mod diag;
#[cfg(feature = "exotic_hash")]
mod hash;
#[cfg(feature = "exotic_hash")]
mod itoken;
mod nameck;
mod parser;
mod scopeck;
mod segment_set;
mod util;
mod verify;

use clap::Arg;
use clap::App;
use database::Database;
use database::DbOptions;
use database::DiagnosticClass;
use diag::Notation;
use std::io;
use std::str::FromStr;

fn positive_integer(val: String) -> Result<(), String> {
    u32::from_str(&val).map(|_| ()).map_err(|e| format!("{}", e))
}

fn main() {
    let matches = App::new("smetamath-rs")
        .version(crate_version!())
        .about("A Metamath database verifier and processing tool")
        .arg(Arg::with_name("DATABASE").help("Database file to load").required_unless("TEXT"))
        .arg(Arg::with_name("split")
            .help("Process files > 1 MiB in multiple segments")
            .long("split"))
        .arg(Arg::with_name("timing").help("Print milliseconds after each stage").long("timing"))
        .arg(Arg::with_name("verify").help("Check proof validity").long("verify").short("v"))
        .arg(Arg::with_name("trace-recalc")
            .help("Print segments as they are recalculated")
            .long("trace-recalc"))
        .arg(Arg::with_name("repeat").help("Demonstrate incremental verifier").long("repeat"))
        .arg(Arg::with_name("jobs")
            .help("Number of threads to use for verification")
            .long("jobs")
            .short("j")
            .takes_value(true)
            .validator(positive_integer))
        .arg(Arg::with_name("TEXT")
            .long("text")
            .help("Provide raw database content on the command line")
            .value_names(&["NAME", "TEXT"])
            .multiple(true))
        .get_matches();

    let mut options = DbOptions::default();
    options.autosplit = matches.is_present("split");
    options.timing = matches.is_present("timing");
    options.verify = matches.is_present("verify");
    options.trace_recalc = matches.is_present("trace-recalc");
    options.incremental = matches.is_present("repeat");
    options.jobs = usize::from_str(matches.value_of("jobs").unwrap_or("1"))
        .expect("validator should check this");

    let mut db = Database::new(options);

    let mut data = Vec::new();
    if let Some(tvals) = matches.values_of_lossy("TEXT") {
        for kv in tvals.chunks(2) {
            data.push((kv[0].clone(), kv[1].clone().into_bytes()));
        }
    }
    let start = matches.value_of("DATABASE")
        .map(|x| x.to_owned())
        .unwrap_or_else(|| data[0].0.clone());

    loop {
        db.parse(start.clone(), data.clone());

        let mut types = vec![
            DiagnosticClass::Parse,
            DiagnosticClass::Scope,
        ];

        if matches.is_present("verify") {
            types.push(DiagnosticClass::Verify);
        }

        for notation in db.diag_notations(types) {
            print_annotation(notation);
        }

        if matches.is_present("repeat") {
            let mut input = String::new();
            if io::stdin().read_line(&mut input).unwrap() == 0 {
                break;
            }
        } else {
            break;
        }
    }
}

fn print_annotation(ann: Notation) {
    let mut args = String::new();
    for (id, val) in ann.args {
        args.push_str(&format!(" {}={}", id, val));
    }
    println!("{}:{}-{}:{:?}:{}{}",
             ann.source.name,
             ann.span.start + ann.source.span.start,
             ann.span.end + ann.source.span.start,
             ann.level,
             ann.message,
             args);
}
