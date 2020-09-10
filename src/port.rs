//! Communication with device ports/interfaces.
//!
//! `lib.rs` deals with general device-level configuration, while this module deals with claiming
//! and communicating with individual ports/interfaces of a device.

use crate::Ftdi;

/// A claimed port on an FTDI device.
///
/// Devices may have anywhere between 1 to 4 ports that can be individually claimed by different
/// applications. A port maps to a USB Interface.
pub struct Port {}

pub struct PortRef<'a> {
    device: &'a Ftdi,
}
