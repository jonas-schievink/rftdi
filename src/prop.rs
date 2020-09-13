//! Device property database.

#[derive(Debug)]
pub(crate) struct DeviceProps {
    /// Device model name.
    pub model: &'static str,
    /// TX buffer size in Bytes.
    pub tx_buf: u16,
    /// RX buffer size in Bytes.
    pub rx_buf: u16,
    /// Data Bits/Pins per port.
    pub port_width: u8,
    /// Port properties.
    pub ports: &'static [PortProps],
}

#[derive(Debug)]
pub(crate) struct PortProps {
    pub mpsse: MpsseSupport,
}

#[derive(Debug)]
pub(crate) enum MpsseSupport {
    /// Low-end devices are fixed converters without MPSSE.
    No,
    /// FT2232C/D have basic MPSSE support.
    ///
    /// Base clock is 12 MHz (effective max. clock is 6 MHz).
    Basic,
    /// `-H` devices support higher clock rates and I²C commands.
    H,
    /// The FT232H supports Open-Collector mode.
    FT232H,
}

static DUMB_PORT: &[PortProps] = &[PortProps {
    mpsse: MpsseSupport::No,
}];

/// Map from `bcdDevice` major version to the device properties (or `None` if that version does not
/// correspond to a known device).
pub(crate) static DEVICES: &[Option<DeviceProps>] = &[
    None, // 0.00
    None, // 1.00
    // 2.00
    Some(DeviceProps {
        model: "FT232AM",
        tx_buf: 128,
        rx_buf: 128,
        port_width: 0, // UART only
        ports: DUMB_PORT,
    }),
    None, // 3.00
    // 4.00
    Some(DeviceProps {
        model: "FT232BM",
        tx_buf: 128,
        rx_buf: 384,
        port_width: 0, // UART only
        ports: DUMB_PORT,
    }),
    // 5.00
    Some(DeviceProps {
        model: "FT2232C/D",
        tx_buf: 128,
        rx_buf: 384,
        port_width: 12, // xDBUS0-7, xCBUS0-3
        ports: &[PortProps {
            mpsse: MpsseSupport::Basic,
        }],
    }),
    // 6.00
    Some(DeviceProps {
        model: "FT232R",
        tx_buf: 256,
        rx_buf: 128,
        port_width: 8,
        ports: DUMB_PORT,
    }),
    // 7.00
    Some(DeviceProps {
        model: "FT2232H",
        tx_buf: 4096,
        rx_buf: 4096,
        port_width: 16, // Has 2 16-bit ports.
        ports: &[
            PortProps {
                mpsse: MpsseSupport::H,
            },
            PortProps {
                mpsse: MpsseSupport::H,
            },
        ],
    }),
    // 8.00
    Some(DeviceProps {
        model: "FT4232H",
        tx_buf: 2048,
        rx_buf: 2048,
        port_width: 8, // Has 4 8-bit ports.
        ports: &[
            PortProps {
                mpsse: MpsseSupport::H,
            },
            PortProps {
                mpsse: MpsseSupport::H,
            },
            PortProps {
                mpsse: MpsseSupport::No,
            },
            PortProps {
                mpsse: MpsseSupport::No,
            },
        ],
    }),
    // 9.00
    Some(DeviceProps {
        model: "FT232H",
        tx_buf: 1024,
        rx_buf: 1024,
        port_width: 16, // Has 1 16-bit port.
        ports: &[PortProps {
            mpsse: MpsseSupport::FT232H,
        }],
    }),
    // 10.00
    Some(DeviceProps {
        model: "FT-X",
        tx_buf: 512,
        rx_buf: 512,
        port_width: 8,
        ports: DUMB_PORT,
    }),
];
