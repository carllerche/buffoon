# Buffoon

A simple Google Protocol Buffers library for Rust.

## Usage

To use `buffoon`, first add this to your `Cargo.toml`:

```toml
[dependencies]
buffoon = "0.5"
```

Then, add this to your crate root:

```rust
extern crate buffoon;
```

## Overview

Buffoon is a simple implementation of the Google Protocol Buffers
library for Rust. It only provides support for reading from and writing
to Protocol Buffer streams. It does not support any code generation or
schema definition.
