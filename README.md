# Rust driver for FTDI USB devices

[![crates.io](https://img.shields.io/crates/v/rftdi.svg)](https://crates.io/crates/rftdi)
[![docs.rs](https://docs.rs/rftdi/badge.svg)](https://docs.rs/rftdi/)
![CI](https://github.com/jonas-schievink/rftdi/workflows/CI/badge.svg)

This crate provides a libusb-based userspace driver library for FTDI USB
interface chips.

Except for libusb, this driver and all dependencies are written in pure Rust and
should work out of the box on all major operating systems (to the extent allowed
by the OS).

Please refer to the [changelog](CHANGELOG.md) to see what changed in the last
releases.

## Usage

Add an entry to your `Cargo.toml`:

```toml
[dependencies]
rftdi = "0.0.0"
```

Check the [API Documentation](https://docs.rs/rftdi/) for how to use the
crate's functionality.

## Rust version support

This crate supports at least the 3 latest stable Rust releases. Bumping the
minimum supported Rust version (MSRV) is not considered a breaking change as
long as these 3 versions are still supported.
