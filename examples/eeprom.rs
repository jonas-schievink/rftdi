//! Access the EEPROM of FTDI devices.

use rftdi::Ftdi;
use std::{error, io, io::Write, process, str::FromStr, time::Duration};
use structopt::StructOpt;

struct UsbIds {
    vid: u16,
    pid: u16,
}

impl FromStr for UsbIds {
    type Err = Box<dyn error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match &*s.split(':').collect::<Vec<_>>() {
            [vid, pid] => {
                let (vid, pid) = (u16::from_str_radix(vid, 16)?, u16::from_str_radix(pid, 16)?);
                Ok(Self { vid, pid })
            }
            _ => Err(format!("USB ID format: `vid:pid`").into()),
        }
    }
}

#[derive(StructOpt)]
struct CommonOpts {
    /// VID:PID of the device to access.
    #[structopt(short = "d")]
    id: Option<UsbIds>,

    /// Write to the EEPROM without asking for confirmation.
    #[structopt(short, long)]
    force: bool,
}

impl CommonOpts {
    fn confirm(&self, what: &str) -> Result<(), Box<dyn error::Error>> {
        if self.force {
            return Ok(());
        }

        eprintln!("WARNING: This will {}. This may brick your device!", what);
        eprint!("Continue? [y/N] ");
        io::stderr().lock().flush()?;
        let mut response = String::new();
        io::stdin().read_line(&mut response)?;
        if response.trim() == "y" {
            Ok(())
        } else {
            Err(format!("operation canceled by user").into())
        }
    }

    fn open(&self) -> Result<Ftdi, Box<dyn error::Error>> {
        let ftdi = match &self.id {
            Some(ids) => Ftdi::open_by_id(ids.vid, ids.pid)?,
            None => Ftdi::open_unique()?,
        };

        Ok(ftdi)
    }
}

#[derive(StructOpt)]
enum Opts {
    Erase {
        #[structopt(flatten)]
        common: CommonOpts,
    },
    Read {
        #[structopt(flatten)]
        common: CommonOpts,

        /// The word address to start reading at (in hexadecimal).
        #[structopt(long, parse(try_from_str = hex_u16), default_value = "0")]
        addr: u16,

        /// The number of words to read.
        #[structopt(long)]
        count: u16,

        /// Output the raw EEPROM contents to stdout instead of printing a hex-dump.
        #[structopt(long)]
        raw: bool,
    },
    Write {
        #[structopt(flatten)]
        common: CommonOpts,

        /// The word address to start writing at (in hexadecimal).
        #[structopt(long, parse(try_from_str = hex_u16), default_value = "0")]
        addr: u16,

        /// The number of words to program.
        #[structopt(long)]
        count: u16,

        /// The 16-bit word to write to the EEPROM `count` times (in hexadecimal).
        #[structopt(long, parse(try_from_str = hex_u16))]
        word: u16,
    },
}

fn hex_u16(s: &str) -> Result<u16, Box<dyn error::Error>> {
    Ok(u16::from_str_radix(s, 16)?)
}

fn main() {
    match run() {
        Ok(()) => {}
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    }
}

fn run() -> Result<(), Box<dyn error::Error>> {
    let opts: Opts = Opts::from_args();

    match opts {
        Opts::Erase { common } => {
            let ftdi = common.open()?;
            common.confirm("erase the EEPROM contents")?;

            eprintln!("Erasing...");
            ftdi.erase_eeprom(Duration::from_secs(5))?;
            eprintln!("EEPROM erased.");
        }
        Opts::Read {
            common,
            addr,
            count,
            raw,
        } => {
            let ftdi = common.open()?;

            if raw {
                let stdout = io::stdout();
                let mut stdout = stdout.lock();
                for a in addr..addr + count {
                    let word = ftdi.read_eeprom_word(a)?;
                    stdout.write_all(&word.to_le_bytes())?;
                }
            } else {
                for (i, a) in (addr..addr + count).enumerate() {
                    let word = ftdi.read_eeprom_word(a)?;

                    if i % 16 == 0 {
                        print!("{:04x}:", a);
                    }

                    print!(" {:04x}", word);

                    if i % 16 == 15 {
                        println!();
                    }
                }
            }
        }
        Opts::Write {
            common,
            addr,
            count,
            word,
        } => {
            let ftdi = common.open()?;

            eprintln!(
                "Writing 0x{:04x} to 0x{:04x}-0x{:04x}",
                word,
                addr,
                addr + count - 1
            );

            common.confirm("write to the EEPROM")?;

            for a in addr..addr + count {
                ftdi.write_eeprom_word(a, word)?;
            }
        }
    }

    Ok(())
}
