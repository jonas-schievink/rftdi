#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
#[repr(u8)]
pub enum BitMode {
    /// RS-232 to USB converter mode, the default after reset.
    Serial = 0x00,
    Bitbang = 0x01,
    Mpsse = 0x02,
    Syncbb = 0x04,
    Mcu = 0x08,
    Opto = 0x10,
    Cbus = 0x20,
    Syncff = 0x40,
}

macro_rules! modes {
    (
        $( $( #[$attr:meta] )* $name:ident;)+
    ) => {
        $(
            $( #[$attr] )*
            #[derive(Debug)]
            pub enum $name {}
        )+

        mod sealed {
            pub trait Sealed {}

            $(
                impl Sealed for super::$name {}
            )+
        }

        pub trait AnyBitMode: sealed::Sealed {
            const MODE: BitMode;
        }

        $(
            impl AnyBitMode for $name {
                const MODE: BitMode = BitMode::$name;
            }
        )+
    };
}

modes! {
    /// RS-232 to USB converter mode, the default after reset.
    Serial;

    Bitbang;

    Mpsse;

    Syncbb;

    Mcu;

    Opto;

    Cbus;

    Syncff;
}
