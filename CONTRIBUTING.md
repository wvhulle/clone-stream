# Contributing

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
