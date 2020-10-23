use rftdi::Ftdi;
use structopt::StructOpt;

use std::io::prelude::*;
use std::{error, io, process};

#[derive(StructOpt)]
struct Opts {
    #[structopt(long)]
    port: Option<u8>,
}

fn main() {
    match run() {
        Ok(()) => {}
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    }
}

fn run() -> Result<(), Box<dyn error::Error>> {
    let opts: Opts = Opts::from_args();

    let ftdi = Ftdi::open_unique()?;
    let mut port = ftdi.open_port(opts.port.unwrap_or(0))?;

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let mut buf = [0; 32];
    loop {
        let n = port.read(&mut buf)?;
        stdout.write_all(&buf[..n])?;
    }
}
