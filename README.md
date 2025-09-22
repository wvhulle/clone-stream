# Clone-Stream

[![Crates.io](https://img.shields.io/crates/v/clone-stream.svg)](https://crates.io/crates/clone-stream)
[![Documentation](https://docs.rs/clone-stream/badge.svg)](https://docs.rs/clone-stream)

Turn any `Stream` into a cloneable stream where each clone receives all items independently.

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

Test benchmarks

```bash
cargo bench --bench fork_clone -- --test
```

Run standard benchmarks:

```bash
cargo bench --bench fork_clone
```

Run all (including comparative) benchmarks:

```bash
cargo bench
```
