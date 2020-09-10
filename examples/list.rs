//! Lists the connected FTDI devices.

use rftdi::Result;
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
    for device in rftdi::devices()? {
        let device = device?;
        println!("{:?}", device);
        device.set_bitmode(0x00)?;
        for addr in 0..16 {
            let word = device.read_eeprom_word(addr)?;
            println!("  0x{:02X}: 0x{:02X}", addr, word);
        }
    }

    Ok(())
}
