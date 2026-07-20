# Bliss Mixer

Simple rust app to serve an HTTP API for mixing songs. This app requires an
existing SQLite database of song analysis as created by [Bliss Analyser](https://github.com/CDrummond/bliss-analyser).

The API served is intended to be used by the [Bliss LMS DSTM plugin](https://github.com/CDrummond/lms-blissmixer).


# Building

Rust 1.97.1 is pinned by `rust-toolchain.toml`. Build with a local rustup
installation or open the repository in its Dev Container for a self-contained
Linux environment with Rust, SQLite tools, and Python.

Build with `cargo build --release`


## Start server

```
$ bliss-mixer
```
