# Clone-Stream

[![Crates.io](https://img.shields.io/crates/v/clone-stream.svg)](https://crates.io/crates/clone-stream)
[![Documentation](https://docs.rs/clone-stream/badge.svg)](https://docs.rs/clone-stream)

Turn any `Stream` into a cloneable stream where each clone receives all items independently.

For more background information, see my [slides](https://github.com/wvhulle/streams-eurorust-2025) for a presentation on EuroRust 2025.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
clone-stream = "0.3"
```

## Documentation

See the [API documentation](https://docs.rs/clone-stream) for examples and usage details.

## Contributing

Run tests:

```bash
cargo test
```

Test run a single benchmark pass:

```bash
cargo bench --bench fork_clone -- --test
```

Run standard statistical benchmarks:

```bash
cargo bench --bench fork_clone
```

Run all (including comparative) benchmarks:

```bash
cargo bench
```
