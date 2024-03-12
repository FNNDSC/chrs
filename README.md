# _ChRIS_ Client (Rust)

[![MIT License](https://img.shields.io/github/license/FNNDSC/chrs)](https://github.com/FNNDSC/chrs/blob/master/LICENSE)
[![Test chrs (bin)](https://github.com/FNNDSC/chrs/actions/workflows/test-chrs.yml/badge.svg)](https://github.com/FNNDSC/chrs/actions/workflows/test-chrs.yml)
[![codecov](https://codecov.io/gh/FNNDSC/chrs/branch/master/graph/badge.svg?token=UOYL5NPYIP)](https://codecov.io/gh/FNNDSC/chrs)

This workspace provides:

- [`chrs`](https://crates.io/crates/chrs), a command-line client for _ChRIS_
  (this is what you're probably looking for)
- [`chris`](https://crates.io/crates/chris), a [Rust](https://www.rust-lang.org/) client library for _ChRIS_
  (this is for developers)

## Development

To set up a development environment, spin up [miniChRIS](https://github.com/FNNDSC/miniChRIS-docker).

[`cargo nextest`](https://nexte.st/) is recommended as an alternative to `cargo test`, and it may also
[be a workaround for a concurrency bug](https://github.com/seanmonstar/reqwest/issues/1148#issuecomment-1453832078).

```shell
cargo nextest run
```

Before committing, remember to lint your code using `cargo fmt` and `cargo clippy`.
