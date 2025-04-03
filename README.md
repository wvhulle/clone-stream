# Forkable streams

The main trait in this library allows you to convert non-cloneable streams into cloneable streams. The only requirement is that the item type of the base-stream is cloneable.





## Usage 

```bash
cargo add --git https://github.com/wvhulle/forked_stream
# Or
cargo add forked_stream

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

This was an exercise in making streams forkable without spawning tasks. First time using `Waker`.

Install `rustup`. Rust unit and integration tests with:

```bash
cargo test
```