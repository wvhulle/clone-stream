# Forkable streams

This Rust library allows you to convert non-cloneable streams into cloneable streams. The only requirement is that the item type of the base-stream is cloneable. The streams are supposed to be Rust types implementing the `Stream` trait from the common `futures` crate.

## Usage 


Stable:

```bash
cargo add forked_stream futures
```

Unstable:

```bash
cargo add --git https://github.com/wvhulle/forked_stream
```

Import the trait `ForkStream` from this crate and call `fork` on your un-cloneable `Stream`:

```rust
use futures::{FutureExt, StreamExt, stream};
use forked_stream::ForkStream;

let uncloneable_stream = stream::iter(0..10);
let cloneable_stream = uncloneable_stream.fork(None);
let mut cloned_stream = cloneable_stream.clone();
```

The required argument for `fork` is the maximum amount of items cached for other pending tasks.


## How does it work?

The stream you start with is called `BaseStream`. You create a kind of adapter / bridge called `ForkBridge`. Then you create forks from this bridge that reference the bridge themselves.

Each time a fork is polled, the waker is used to check whether this task already polled the bridge in the past. If the fork already polled, it will be in a list on the bridge and may contain buffered items not seen yet.

## Contributing

This small project was an exercise for me making streams forkable without spawning tasks. This is my first time storing and managing `Waker` objects. Sorry for any mistakes.

There are a few alternative solutions on `crates.io`.

You can run the tests with:

```bash
cargo test
```
