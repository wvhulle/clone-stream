# Forkable streams

An exercise in making streams forkable without spawning tasks.


## Usage 

```bash
cargo add --git https://github.com/wvhulle/forked_stream
```

Then import the trait `ForkStream` from this crate and call `fork` on your `Stream`:

```rust
use forked_stream::ForkStream;
let cloneable_stream = uncloneable_stream.fork();
cloneable_stream.clone();
```


## Contributing


Install `rustup`.

```bash
cargo test
```