# chrs

[![crates.io version](https://img.shields.io/crates/v/chrs?label=version)](https://crates.io/crates/chrs)
[![codecov](https://codecov.io/gh/FNNDSC/chrs/branch/master/graph/badge.svg?flag=chrs&token=UOYL5NPYIP)](https://codecov.io/gh/FNNDSC/chrs)
[![Publish](https://github.com/FNNDSC/chrs/actions/workflows/release.yml/badge.svg)](https://github.com/FNNDSC/chrs/actions/workflows/release.yml)

`chrs` is a command-line client for
[_ChRIS_](https://chrisproject.org).

## Installation

There are four ways to install `chrs`.

### Direct Download

You can download `chrs` from
[GitHub Releases](https://github.com/FNNDSC/chrs/releases).
Get the latest version here:

https://github.com/FNNDSC/chrs/releases/latest

This is the easiest installation method, however there is no
mechanism for automatic updates.

### Using Pip

`chrs` is published to [PyPi](https://pypi.org/project/chrs) using
[PyO3/maturin](https://github.com/PyO3/maturin). Installing `chrs`
using `pip` will be preferable for users who are already setup with Python.

```shell
pip install --user chrs
```

If necessary, add the `bin` folder to `$PATH`:

```shell
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### `cargo binstall`

[`cargo binstall`](https://github.com/cargo-bins/cargo-binstall) is a convenient solution
for installing pre-compiled binaries using `cargo` as a package manager. First install rust-binstall,
then run

```shell
cargo binstall chrs
```

### Build from source from crates.io

Use [cargo](https://doc.rust-lang.org/cargo/) to get and build the
package from source, from crates.io:

```shell
cargo install chrs
```

If necessary, add the `bin` folder to `$PATH`:

```shell
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

## Using `chrs`

Run `chrs --help` for usage information.

Note: when specifying URLs, they should be to the backend API, not the front-end.
E.g. instead of `https://app.chrisproject.org/feeds/1520`, the correct URL would
be `https://cube.chrisproject.org/api/v1/1520/`.

### Overview

TODO TODO TODO

### Feedback

Please report bugs and/or request features here:
https://github.com/FNNDSC/chrs/issues
