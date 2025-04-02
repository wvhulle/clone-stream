# Forkable streams

An exercise in making streams forkable without spawning tasks.


## Usage 

```bash
cargo add --git https://github.com/wvhulle/forked_stream
cargo add futures
```

Then import the trait `ForkStream` from this crate and call `fork` on your `Stream`:

```rust
use futures::{FutureExt, StreamExt, stream};

fn main() {
    use forked_stream::ForkStream;

    let uncloneable_stream = stream::iter(0..10);
    let cloneable_stream = uncloneable_stream.fork();
    let mut cloned_stream = cloneable_stream.clone();

    let first = cloned_stream
        .next()
        .now_or_never()
        .expect("cloned stream should yield immediately")
        .expect("all items should be `Some`.");

    println!("First item: {}", first);
}

```


## Contributing


Install `rustup`.

```bash
cargo test
```