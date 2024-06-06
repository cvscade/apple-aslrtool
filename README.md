# apple-aslrtool ![Crates.io Version](https://img.shields.io/crates/v/apple-aslrtool) ![Crates.io License](https://img.shields.io/crates/l/apple-aslrtool)

Simple tool to fetch the ASLR slide for a given process on Apple Silicon and Intel Mac systems.

# Features

- Detect hardened runtime
    - Note: If you have SIP disabled, this will always return `false` as the hardened runtime is enforced by SIP.
    - You can remove hardened runtime from a binary by removing its signature using `codesign --remove-signature`.
- Cross-architecture support (x86_64 binary can pull slide of aarch64 task and vice versa) 
    - Note: **YOUR MILEAGE MAY VARY!** arm64e is not officially supported and may require additional work on your part.

# Usage

- `apple-aslrtool --pid=<pid>`: Fetch the ASLR slide for the given process using a PID.
- `apple-aslrtool --name=<pid>`: Fetch the ASLR slide for the given process using a name. The first found task will be used. **Any tasks using hardened runtime will be ignored!**

You can also provide an additional `--base-address` flag in case you want to override the default `0x100000000` value.

# Installation

- `cargo install apple-aslrtool`
- `cargo binstall apple-aslrtool`

# Requirements

- x86_64-apple-darwin build requires macOS 10.7 or later (untested on Intel hardware)
- aarch64-apple-darwin build requires macOS >= 11.0 (tested on macOS 14 only)
