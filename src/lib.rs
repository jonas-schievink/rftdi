//! TODO: Write crate docs

#![doc(html_root_url = "https://docs.rs/rftdi/0.0.0")]
// Deny a few warnings in doctests, since rustdoc `allow`s many warnings by default
#![doc(test(attr(deny(unused_imports, unused_must_use))))]
#![warn(missing_debug_implementations, rust_2018_idioms)]

mod error;
mod port;
mod prop;
mod readme;

use prop::DeviceProps;
use std::{fmt, time::Duration};

pub use self::error::{Error, ErrorKind};

pub type Result<T> = std::result::Result<T, Error>;

/// FTDI's USB vendor ID.
pub const VID_FTDI: u16 = 0x0403;

/// Product IDs used by FTDI's official devices.
pub const PIDS_FTDI: &[u16] = &[0x6001, 0x6010, 0x6011, 0x6015];

#[allow(unused)]
#[repr(u8)]
enum ControlReq {
    Reset = 0x00,
    SetModemCtrl = 0x01,
    SetFlowCtrl = 0x02,
    SetBaudrate = 0x03,
    SetData = 0x04,
    PollModemStatus = 0x05,
    SetEventChar = 0x06,
    SetErrorChar = 0x07,
    SetLatencyTimer = 0x09,
    GetLatencyTimer = 0x0A,
    SetBitmode = 0x0B,
    ReadPins = 0x0C,
    ReadEeprom = 0x90,
    WriteEeprom = 0x91,
    EraseEeprom = 0x92,
}

pub struct Ftdi {
    device: rusb::DeviceHandle<rusb::GlobalContext>,
    timeout: Duration,
    properties: &'static DeviceProps,
}

const REQ_TYPE_VENDOR: u8 = 0x02 << 5;
const REQ_RECIPIENT_DEVICE: u8 = 0x00;
const REQ_DIR_OUT: u8 = 0x00;
const REQ_DIR_IN: u8 = 0x80;

impl Ftdi {
    const DEFAULT_TIMEOUT: Duration = Duration::from_millis(500);
    const REQ_READ: u8 = REQ_TYPE_VENDOR | REQ_RECIPIENT_DEVICE | REQ_DIR_IN;
    const REQ_WRITE: u8 = REQ_TYPE_VENDOR | REQ_RECIPIENT_DEVICE | REQ_DIR_OUT;

    /// Opens the only FTDI device connected to the system.
    pub fn open_unique() -> Result<Self> {
        Self::open_filtered(|dev| {
            let descr = dev.device_descriptor()?;
            Ok(descr.vendor_id() == VID_FTDI && PIDS_FTDI.contains(&descr.product_id()))
        })
    }

    /// Opens an FTDI device with the given VID and PID.
    ///
    /// If multiple devices match the IDs, an error will be returned.
    pub fn open_by_id(vid: u16, pid: u16) -> Result<Self> {
        Self::open_filtered(|dev| {
            let descr = dev.device_descriptor()?;
            Ok(descr.vendor_id() == vid && descr.product_id() == pid)
        })
    }

    /// Opens a device by its unique USB address.
    ///
    /// This address in unique per system, so only a single USB device can match. The address is not
    /// device-specific and may change when the device is replugged.
    pub fn open_by_addr(bus_number: u8, device_address: u8) -> Result<Self> {
        Self::open_filtered(|dev| {
            Ok(dev.bus_number() == bus_number && dev.address() == device_address)
        })
    }

    /// Opens a unique device that matches a `filter` predicate.
    ///
    /// Private, since we don't want to make rusb a public dependency.
    fn open_filtered(
        mut filter: impl FnMut(&rusb::Device<rusb::GlobalContext>) -> Result<bool>,
    ) -> Result<Self> {
        let list = rusb::devices()?;
        let mut selected_device = None;
        for device in list.iter() {
            if filter(&device)? {
                if selected_device.is_some() {
                    return Err(ErrorKind::MultipleDevicesFound.into());
                }
                selected_device = Some(device);
            }
        }

        match selected_device {
            Some(device) => Self::open(device),
            None => Err(ErrorKind::NoDeviceFound.into()),
        }
    }

    fn open(device: rusb::Device<rusb::GlobalContext>) -> Result<Self> {
        let descr = device.device_descriptor()?;
        let version = descr.device_version();
        if version.minor() != 0 || version.sub_minor() != 0 {
            return Err(ErrorKind::UnsupportedDevice.into());
        }

        let properties = match prop::DEVICES.get(version.major() as usize) {
            Some(Some(props)) => props,
            _ => return Err(ErrorKind::UnsupportedDevice.into()),
        };

        let device = device.open()?;
        Ok(Self {
            device,
            properties,
            timeout: Self::DEFAULT_TIMEOUT,
        })
    }

    fn dev_descr(&self) -> rusb::DeviceDescriptor {
        // This is infallible since libusb 1.0.16, which is from 2013, so we just assume that it
        // won't fail.
        self.device.device().device_descriptor().unwrap()
    }

    /// Returns the USB Product ID of this device.
    pub fn pid(&self) -> u16 {
        self.dev_descr().product_id()
    }

    /// Returns the USB Vendor ID of this device.
    pub fn vid(&self) -> u16 {
        self.dev_descr().vendor_id()
    }

    /// Returns the USB bus number this device is attached to.
    ///
    /// Alongside `device_address()`, this uniquely identifies a device connected to the system.
    pub fn bus_number(&self) -> u8 {
        self.device.device().bus_number()
    }

    /// Returns the USB address assigned to the device on its bus.
    ///
    /// Alongside `bus_number()`, this uniquely identifies a device connected to the system.
    pub fn device_address(&self) -> u8 {
        self.device.device().address()
    }

    /// Reads the serial number string from the device.
    ///
    /// Most FTDI devices do not have unique serial number strings, so this cannot be used to
    /// identify devices.
    pub fn serial(&self) -> Result<String> {
        let descr = self.device.device().device_descriptor()?;
        Ok(self.device.read_serial_number_string_ascii(&descr)?)
    }

    /// Reads the product description string from the device.
    pub fn product(&self) -> Result<String> {
        Ok(self.device.read_product_string_ascii(&self.dev_descr())?)
    }

    /// Returns the device model name.
    ///
    /// This is looked up using the version reported by the device, which uniquely identifies
    /// different generations of products.
    pub fn model(&self) -> &str {
        self.properties.model
    }

    /// Resets the USB device.
    pub fn reset_device(&mut self) -> Result<()> {
        Ok(self.device.reset()?)
    }

    pub fn set_bitmode(&self, mode: u8) -> Result<()> {
        self.device.write_control(
            Self::REQ_WRITE,
            ControlReq::SetBitmode as u8,
            (mode as u16) << 8 | 0x00,
            1, // bInterfaceNumber + 1
            &[],
            self.timeout,
        )?;
        Ok(())
    }

    pub fn bitmode(&self) -> Result<u8> {
        let mut buf = [0; 4];
        let n = self.device.read_control(
            Self::REQ_READ,
            ControlReq::ReadPins as u8,
            1,
            1,
            &mut buf,
            self.timeout,
        )?;
        assert_eq!(n, 1);

        Ok(buf[0])
    }

    pub fn read_eeprom_word(&self, word_addr: u16) -> Result<u16> {
        let mut buf = [0; 2];
        let n = self.device.read_control(
            Self::REQ_READ,
            ControlReq::ReadEeprom as u8,
            0,
            word_addr,
            &mut buf,
            self.timeout,
        )?;
        assert_eq!(n, 2);
        Ok(u16::from_le_bytes(buf))
    }

    pub fn write_eeprom_word(&self, word_addr: u16, word: u16) -> Result<()> {
        let n = self.device.write_control(
            Self::REQ_WRITE,
            ControlReq::WriteEeprom as u8,
            word,
            word_addr,
            &mut [],
            self.timeout,
        )?;
        assert_eq!(n, 0);
        Ok(())
    }

    pub fn erase_eeprom(&self, timeout: Duration) -> Result<()> {
        let n = self.device.write_control(
            Self::REQ_WRITE,
            ControlReq::EraseEeprom as u8,
            0,
            0,
            &mut [],
            timeout,
        )?;
        assert_eq!(n, 0);
        Ok(())
    }

    pub fn num_ports(&self) -> Result<u8> {
        let conf = self.device.active_configuration()?;
        let descr = self.device.device().config_descriptor(conf)?;
        Ok(descr.num_interfaces())
    }
}

impl fmt::Debug for Ftdi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ftdi")
            .field("properties", &self.properties)
            .finish()
    }
}

/// Returns an iterator that will try to open every FTDI device on the system.
///
/// This will open every device whose VID and PID match the official FTDI products.
pub fn devices() -> Result<impl Iterator<Item = Result<Ftdi>>> {
    devices_filtered(|dev| {
        let descr = dev.device_descriptor()?;
        Ok(descr.vendor_id() == VID_FTDI && PIDS_FTDI.contains(&descr.product_id()))
    })
}

pub fn devices_by_id(vid: u16, pid: u16) -> Result<impl Iterator<Item = Result<Ftdi>>> {
    devices_filtered(move |dev| {
        let descr = dev.device_descriptor()?;
        Ok(descr.vendor_id() == vid && descr.product_id() == pid)
    })
}

fn devices_filtered(
    mut filter: impl FnMut(&rusb::Device<rusb::GlobalContext>) -> Result<bool>,
) -> Result<impl Iterator<Item = Result<Ftdi>>> {
    let list = rusb::devices()?;
    let mut vec = Vec::new();
    for device in list.iter() {
        match filter(&device) {
            Ok(true) => {
                let ftdi = Ftdi::open(device);
                vec.push(ftdi.map_err(Into::into));
            }
            Ok(false) => {}
            Err(e) => {
                vec.push(Err(e));
            }
        }
    }

    Ok(vec.into_iter())
}
