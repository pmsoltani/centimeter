# centimeter

`centimeter` is a robust, mathematically rigorous double-entry accounting engine written in Rust.

Built to be embedded everywhere, it provides a minimal, bulletproof core for financial truth, paired with an open ecosystem of extensions for domain-specific accounting needs.

> **Status:** Early development. Stay tuned!

## Key Principles

- **A Tiny, Closed Core:** The core engine enforces one mathematical invariant: `∑ value = 0`. If a feature does not participate in balancing (like invoicing, taxes, or parties), it is layered outside the core as an extension.
- **Exact Decimal Numerics:** Zero floating-point arithmetic. `centimeter` uses exact scaled decimals with per-commodity precision.
- **Multi-Commodity Native:** Currency, crypto, shares, hours, and physical units share a single, unified code path.
- **Append-Only & Tamper-Evident:** Posted transactions are immutable and corrected strictly by appending. The core supports cryptographic hash chains and injected provenance for statutory-grade audit trails.
- **Extensible by Design:** Extensions attach domain data (like counterparties or reconciliation statuses) via stable TypeID side relations, ensuring the core remains ignorant of outside concerns.

## Workspace Structure

The project is structured as a Cargo virtual workspace to enforce strict dependency boundaries:

```text
crates/
  centimeter/   # The facade crate, re-exporting the core and chosen extensions
  core/         # The closed accounting engine (model, balancing, lifecycle)
  ...           # First-party extensions
apps/           # Composition roots (CLI, Server) that wire the core and extensions together
bindings/       # Standalone packages for FFI (Python, Node, C, WASM)
fuzz/           # Standalone workspace for cargo-fuzz targets
```

_Note: Consumers should depend on the `centimeter` facade crate, which safely re-exports the core and chosen extensions behind feature flags._

## Documentation

The design of `centimeter` is documented. If you are exploring the codebase, start here:

- [Architecture Decision Records (ADRs)](docs/adr/README.md)
- [Glossary](docs/glossary.md)

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/>LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
