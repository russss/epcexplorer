# EPC Explorer

A tool to explore UHF RFID tags, written in Rust. Electronic Product Code (EPC) fields are decoded using the [GS1](https://crates.io/crates/gs1) crate.

![Screenshot](/img/screenshot.png)

## Supported Readers

Currently only the CH-RU5102 reader is supported.

## Installing

Assuming you have the Rust toolchain installed, you can install with :

	$ cargo install epcexplorer

## Running

Pass the serial device name to the binary - in my case:

	$ epcexplorer /dev/cu.SLAB_USBtoUART
