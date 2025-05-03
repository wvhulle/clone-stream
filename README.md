# Clone-stream

Convert any stream into a stream that is `Clone`. The only requirement is that the item type of the base-stream implements `Clone`.


_Remark: A stream is an implementor of the `Stream` trait from the `futures` crate (also called an async iterator)._

## Installation

In an existing `cargo` project:

```bash
cargo add clone-stream futures
```

If you would like to install the latest `git` version instead of the latest official release on [crates.io](https://crates.io/crates/clone-stream), you can add this crate as a `git` dependency:


```bash
cargo add --git https://github.com/wvhulle/clone-stream
```

## Usage

See the [docs](https://docs.rs/clone-stream/latest/clone_stream/) for the API documentation.


## How does it work?

The Rust integration tests in [tests](./tests) show some examples.

See [my blog](https://willemvanhulle.tech/blog) for more technical information.