# Forkable streams

This library allows you to convert non-cloneable streams into cloneable streams. The only requirement is that the item type of the base-stream is cloneable.

## Usage 


Stable:

```bash
cargo add forked_stream
cargo add futures
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
let cloneable_stream = uncloneable_stream.fork();
let mut cloned_stream = cloneable_stream.clone();
```


## Contributing

This was an exercise in making streams forkable without spawning tasks. First time using `Waker`.

Install `rustup`. Rust unit and integration tests with:

```bash
cargo test
```