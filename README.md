# Rust library `clone-stream`

This Rust library allows you to convert non-cloneable streams into cloneable streams. The only requirement is that the item type of the base-stream is cloneable. The streams are supposed to be Rust types implementing the `Stream` trait from the common `futures` crate.

## Installation

Make sure you have installed `rustup`, which installs `cargo`. Inside an existing `cargo` project:

```bash
cargo add clone-stream futures
```

If you would like to install the latest `git` version instead of the release on [crates.io](crates.io):

```bash
cargo add --git https://github.com/wvhulle/clone-stream
```

## Usage

Import the trait `ForkStream` from this crate and call `split` on your un-cloneable `Stream`:

```rust
use futures::{FutureExt, StreamExt, stream};
use clone_stream::ForkStream;

let uncloneable_stream = stream::iter(0..10);
let cloneable_stream = uncloneable_stream.split();
let mut cloned_stream = cloneable_stream.clone();
```



## How does it work?

This small project was an exercise for me making streams implement `Clone` without spawning tasks. 

See [my blog](willemvanhulle.tech) for more technical information.