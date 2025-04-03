# Forkable streams

This library allows you to convert non-cloneable streams into cloneable streams. The only requirement is that the item type of the base-stream is cloneable.

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

The required argument for `fork` is the size of the buffers allocated for each separate suspended awaiting task. 

Remark: The extra argument is necessary because it is possible that other task are scheduled to run earlier. They may poll new items from the base stream more quickly and the current suspended task might not be able to catch up.

## Contributing

This was an exercise in making streams forkable without spawning tasks. This is my first time storing and managing `Waker` objects. Sorry for any mistakes.

Run Rust unit and integration tests with:

```bash
cargo test
```