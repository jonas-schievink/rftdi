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
use std::{cell::RefCell, cell::RefMut, fmt, rc::Rc, time::Duration};

pub use error::{Error, ErrorKind};
pub use port::Port;

/// A result type with the error hardwired to [`Error`].
///
/// [`Error`]: struct.Error.html
pub type Result<T> = std::result::Result<T, Error>;

/// FTDI's USB vendor ID.
pub const VID_FTDI: u16 = 0x0403;

/// Product IDs used by FTDI's official devices.
pub const PIDS_FTDI: &[u16] = &[0x6001, 0x6010, 0x6011, 0x6015];

/// USB device type providing shared access from multiple ports.
type UsbHandle = Rc<RefCell<rusb::DeviceHandle<rusb::GlobalContext>>>;

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

/// An FTDI USB device.
pub struct Ftdi {
    device: UsbHandle,
    timeout: Duration,
    properties: &'static DeviceProps,
}

const REQ_TYPE_VENDOR: u8 = 0x02 << 5;
const REQ_RECIPIENT_DEVICE: u8 = 0x00;
const REQ_DIR_OUT: u8 = 0x00;
const REQ_DIR_IN: u8 = 0x80;

const REQ_READ: u8 = REQ_TYPE_VENDOR | REQ_RECIPIENT_DEVICE | REQ_DIR_IN;
const REQ_WRITE: u8 = REQ_TYPE_VENDOR | REQ_RECIPIENT_DEVICE | REQ_DIR_OUT;

impl Ftdi {
    const DEFAULT_TIMEOUT: Duration = Duration::from_millis(500);

    /// Opens the only FTDI device connected to the system.
    pub fn open_unique() -> Result<Self> {
        Self::open_filtered(|dev| {
            let descr = dev.device_descriptor().map_err(Error::usb)?;
            Ok(descr.vendor_id() == VID_FTDI && PIDS_FTDI.contains(&descr.product_id()))
        })
    }

    /// Opens an FTDI device with the given VID and PID.
    ///
    /// If multiple devices match the IDs, an error will be returned.
    pub fn open_by_id(vid: u16, pid: u16) -> Result<Self> {
        Self::open_filtered(|dev| {
            let descr = dev.device_descriptor().map_err(Error::usb)?;
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
        let list = rusb::devices().map_err(Error::usb)?;
        let mut selected_device = None;
        for device in list.iter() {
            if filter(&device)? {
                if selected_device.is_some() {
                    return Err(Error::from_kind(ErrorKind::MultipleDevicesFound));
                }
                selected_device = Some(device);
            }
        }

        match selected_device {
            Some(device) => Self::open(device),
            None => Err(Error::from_kind(ErrorKind::NoDeviceFound)),
        }
    }

    fn open(device: rusb::Device<rusb::GlobalContext>) -> Result<Self> {
        log::debug!("Ftdi::open(device = {:?})", device);

        let descr = device.device_descriptor().map_err(Error::usb)?;

        if descr.num_configurations() != 1 {
            log::error!(
                "device has {} configurations, expected 1",
                descr.num_configurations()
            );
            return Err(Error::from_kind(ErrorKind::UnsupportedDevice));
        }

        let conf_descr = device.active_config_descriptor().map_err(Error::usb)?;

        // Every interface must have vendor descriptors and a pair of bulk endpoints.
        for intf in conf_descr.interfaces() {
            let mut iter = intf.descriptors();
            let descr = iter.next();

            match descr {
                Some(descr) => if descr.num_endpoints() != 2 {},
                None => {
                    log::error!("missing interface descriptor");
                    return Err(Error::from_kind(ErrorKind::UnsupportedDevice));
                }
            }

            if iter.next().is_some() {
                log::error!("found extra interface descriptor");
                return Err(Error::from_kind(ErrorKind::UnsupportedDevice));
            }
        }

        let version = descr.device_version();
        if version.minor() != 0 || version.sub_minor() != 0 {
            return Err(Error::from_kind(ErrorKind::UnsupportedDevice));
        }

        let properties = match prop::DEVICES.get(version.major() as usize) {
            Some(Some(props)) => props,
            _ => return Err(Error::from_kind(ErrorKind::UnsupportedDevice)),
        };

        if usize::from(conf_descr.num_interfaces()) != properties.ports.len() {
            log::error!(
                "device reports {} interfaces, expected {}",
                conf_descr.num_interfaces(),
                properties.ports.len()
            );
            return Err(Error::from_kind(ErrorKind::UnsupportedDevice));
        }

        let device = device.open().map_err(|e| {
            if cfg!(windows) && matches!(e, rusb::Error::NotSupported | rusb::Error::NotFound) {
                // Provide a more helpful error message on non-plug-and-play platforms.
                Error::new(
                    ErrorKind::Usb,
                    format!(
                        "{} (this error may be caused by not having the WinUSB driver installed; \
                            use Zadig (https://zadig.akeo.ie/) to install it for the FTDI device; \
                            this will replace any existing driver)",
                        e
                    ),
                )
            } else {
                Error::new(ErrorKind::Usb, e)
            }
        })?;

        Ok(Self {
            device: Rc::new(RefCell::new(device)),
            properties,
            timeout: Self::DEFAULT_TIMEOUT,
        })
    }

    fn dev(&self) -> RefMut<'_, rusb::DeviceHandle<rusb::GlobalContext>> {
        self.device.borrow_mut()
    }

    fn dev_descr(&self) -> rusb::DeviceDescriptor {
        // This is infallible since libusb 1.0.16, which is from 2013, so we just assume that it
        // won't fail.
        self.dev().device().device_descriptor().unwrap()
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
        self.dev().device().bus_number()
    }

    /// Returns the USB address assigned to the device on its bus.
    ///
    /// Alongside `bus_number()`, this uniquely identifies a device connected to the system.
    pub fn device_address(&self) -> u8 {
        self.dev().device().address()
    }

    /// Reads the serial number string from the device.
    ///
    /// Most FTDI devices do not have unique serial number strings, so this cannot be used to
    /// identify devices.
    pub fn serial(&self) -> Result<String> {
        let descr = self.dev_descr();
        Ok(self
            .dev()
            .read_serial_number_string_ascii(&descr)
            .map_err(Error::usb)?)
    }

    /// Reads the product description string from the device.
    pub fn product(&self) -> Result<String> {
        let descr = self.dev_descr();
        Ok(self
            .dev()
            .read_product_string_ascii(&descr)
            .map_err(Error::usb)?)
    }

    /// Returns the FTDI model identification.
    ///
    /// This is looked up using the version reported by the device, which uniquely identifies
    /// different generations of products.
    pub fn model(&self) -> &str {
        self.properties.model
    }

    /// Resets the USB device.
    pub fn reset_device(&mut self) -> Result<()> {
        Ok(self.dev().reset().map_err(Error::usb)?)
    }

    /// Returns the configured timeout for USB operations.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Sets the timeout to use for USB operations.
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// Reads a 16-bit word from the EEPROM.
    ///
    /// The caller has to ensure that the word address is in bounds, or the returned value is
    /// meaningless.
    pub fn read_eeprom_word(&self, word_addr: u16) -> Result<u16> {
        let mut buf = [0; 2];
        let n = self
            .dev()
            .read_control(
                REQ_READ,
                ControlReq::ReadEeprom as u8,
                0,
                word_addr,
                &mut buf,
                self.timeout,
            )
            .map_err(Error::usb)?;
        assert_eq!(n, 2);
        Ok(u16::from_le_bytes(buf))
    }

    /// Writes a 16-bit word to the EEPROM.
    ///
    /// **Warning**: This can overwrite the device configuration, which can brick the device. Use
    /// with caution!
    ///
    /// The caller has to ensure that the word address is in bounds, or this operation might
    /// misbehave (eg. by writing to unintended EEPROM locations, or by not writing data at all).
    pub fn write_eeprom_word(&self, word_addr: u16, word: u16) -> Result<()> {
        let n = self
            .dev()
            .write_control(
                REQ_WRITE,
                ControlReq::WriteEeprom as u8,
                word,
                word_addr,
                &mut [],
                self.timeout,
            )
            .map_err(Error::usb)?;
        assert_eq!(n, 0);
        Ok(())
    }

    /// Erases the attached EEPROM.
    ///
    /// **Warning**: This can erase the device configuration, which can brick the device. Use with
    /// caution!
    ///
    /// This will not use the USB timeout configured with `set_timeout`. Instead, the `timeout`
    /// parameter will be used, since EEPROM erasure may take longer than other operations.
    pub fn erase_eeprom(&self, timeout: Duration) -> Result<()> {
        let n = self
            .dev()
            .write_control(
                REQ_WRITE,
                ControlReq::EraseEeprom as u8,
                0,
                0,
                &mut [],
                timeout,
            )
            .map_err(Error::usb)?;
        assert_eq!(n, 0);
        Ok(())
    }

    /// Returns the number of ports this device has.
    pub fn num_ports(&self) -> u8 {
        self.properties.ports.len() as u8
    }

    /// Opens a port of this device.
    ///
    /// This will claim the corresponding USB interface and lock it for other applications.
    ///
    /// The port will inherit `self`'s USB timeout.
    ///
    /// # Parameters
    ///
    /// * **`port`**: The port index. Must be in range `0..self.num_ports()`, or this method will
    ///   panic.
    pub fn open_port(&self, port: u8) -> Result<Port> {
        assert!(
            port < self.num_ports(),
            "port {} out of range (device only has {})",
            port,
            self.num_ports()
        );

        Port::open(self, port)
    }
}

impl fmt::Debug for Ftdi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ftdi")
            .field("timeout", &self.timeout)
            .field("properties", &self.properties)
            .finish()
    }
}

/// Returns an iterator over all FTDI devices on the system.
///
/// This will try to open every device whose VID and PID match known FTDI products.
pub fn devices() -> Result<impl Iterator<Item = Result<Ftdi>>> {
    devices_filtered(|dev| {
        let descr = dev.device_descriptor().map_err(Error::usb)?;
        Ok(descr.vendor_id() == VID_FTDI && PIDS_FTDI.contains(&descr.product_id()))
    })
}

/// Returns an iterator over all devices matching the given IDs.
pub fn devices_by_id(vid: u16, pid: u16) -> Result<impl Iterator<Item = Result<Ftdi>>> {
    devices_filtered(move |dev| {
        let descr = dev.device_descriptor().map_err(Error::usb)?;
        Ok(descr.vendor_id() == vid && descr.product_id() == pid)
    })
}

fn devices_filtered(
    mut filter: impl FnMut(&rusb::Device<rusb::GlobalContext>) -> Result<bool>,
) -> Result<impl Iterator<Item = Result<Ftdi>>> {
    let list = rusb::devices().map_err(Error::usb)?;
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
