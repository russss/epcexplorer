# EPC Explorer

A tool to explore UHF RFID tags, written in Rust. Electronic Product Code (EPC) fields are decoded using the [GS1](https://crates.io/crates/gs1) crate.

![Screenshot](/img/screenshot.png)

## Supported Readers

* CH-RU5102 (`ru5102` driver)
* Invelion and similar (`invelion` driver)

## Installing

Assuming you have the Rust toolchain installed, you can install with :

	$ cargo install epcexplorer

## Running

Pass the serial device name and driver name to the binary - in my case:

	$ epcexplorer /dev/cu.SLAB_USBtoUART ru5102
