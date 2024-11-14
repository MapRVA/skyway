# Installation

## Pre-compiled Binaries

Builds of skyway are available on the [releases page](https://github.com/MapRVA/skyway/releases).
Feel free to get in touch if you'd like me to prepare a binary for your platform.

## Install Using Cargo

skyway is [on crates.io](https://crates.io/crates/skyway), so you can install it using cargo:

```sh
cargo install skyway
```

Make sure your cargo bin directory is in your $PATH.

## Selecting Features

> Cargo "features" provide a mechanism to express conditional compilation and optional dependencies.
> A package defines a set of named features in the [features] table of Cargo.toml, and each feature can either be enabled or disabled.
> Features for the package being built can be enabled on the command-line with flags such as --features.
> Features for dependencies can be enabled in the dependency declaration in Cargo.toml.

―[The Cargo Book](https://doc.rust-lang.org/cargo/reference/features.html)

skyway's `Cargo.toml` file defines [features](https://doc.rust-lang.org/cargo/reference/features.html) which allow you to selectively compile certain file or filter formats.
By default, all features are enabled.
As well as providing a feature name for each individual file and filter format, filtering support can be enabled/disabled using the `"filter"` feature.
Enabling the `"cel"` feature, for example, will automatically enable `"filter"`, but you can enable `"filter"` by itself to access generic filtering functionality.
Disabling all features leaves a relatively small library of OpenStreetMap conversion tools.
