# Forkable streams

This Rust library allows you to convert non-cloneable streams into cloneable streams. The only requirement is that the item type of the base-stream is cloneable. The streams are supposed to be Rust types implementing the `Stream` trait from the common `futures` crate.

## Installation

Make sure you have installed `rustup`, which installs `cargo`. Inside an existing `cargo` project:

```bash
cargo add forked_stream futures
```

If you would like to install the latest `git` version instead of the release on [crates.io](crates.io):

```bash
cargo add --git https://github.com/wvhulle/forked_stream
```

## Usage

Import the trait `ForkStream` from this crate and call `fork` on your un-cloneable `Stream`:

```rust
use futures::{FutureExt, StreamExt, stream};
use forked_stream::ForkStream;

let uncloneable_stream = stream::iter(0..10);
let cloneable_stream = uncloneable_stream.fork();
let mut cloned_stream = cloneable_stream.clone();
```


## Contributing

You can run the tests with:

```bash
cargo test
```


## Author remarks

This small project was an exercise for me making streams forkable without spawning tasks. This is my first time storing and managing `Waker` objects. Sorry for any mistakes.

There are a few alternative solutions on `crates.io`.