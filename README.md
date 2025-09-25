# Clone-Stream

[![Crates.io](https://img.shields.io/crates/v/clone-stream.svg)](https://crates.io/crates/clone-stream)
[![Documentation](https://docs.rs/clone-stream/badge.svg)](https://docs.rs/clone-stream)

Turn any `Stream` into a cloneable stream where each clone receives all items independently.

For more background information, see my [slides](https://github.com/wvhulle/streams-eurorust-2025) for a presentation on EuroRust 2025.

## Installation

```bash
cargo add clone-stream
```

## Documentation

See the [API documentation](https://docs.rs/clone-stream) for examples and usage details.

## Contributing

Run tests:

```bash
cargo test
```

Use the `clean_log::log` function for simple display of log messages in tests.

Test run a single benchmark pass:

```bash
cargo bench --bench fork_clone -- --test
```

Run standard statistical benchmarks (with detailed output):

```bash
cargo bench --bench fork_clone
```

Run all benchmarks (without performance change output):

```bash
cargo bench
```
