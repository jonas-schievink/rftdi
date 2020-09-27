//! Lists the connected FTDI devices.

use rftdi::{Ftdi, Result};
use std::process;

fn main() {
    match run() {
        Ok(()) => {}
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    }
}

fn run() -> Result<()> {
    env_logger::init();

    for device in rftdi::devices()? {
        match device.and_then(|device| dump_device(&device)) {
            Ok(()) => {}
            Err(err) => {
                eprintln!("{}", err);
            }
        }
    }

    Ok(())
}

fn dump_device(device: &Ftdi) -> Result<()> {
    println!(
        "Bus {:03} Address {:03}: ID {:04x}:{:04x} {} ({:?})",
        device.bus_number(),
        device.device_address(),
        device.vid(),
        device.pid(),
        device.model(),
        device.product()?,
    );

    for port_num in 0..device.num_ports() {
        print!("  Port {}:", port_num);
        let port = device.open_port(port_num)?;
        println!(" {:?}", port);
        println!("    Pin state: 0b{:08b}", port.read_pins()?);
    }

    Ok(())
}
