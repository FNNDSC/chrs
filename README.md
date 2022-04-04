# _ChRIS_ Client (Rust)

[![crates.io version](https://img.shields.io/crates/v/chrs?label=version)](https://crates.io/crates/chrs)
[![MIT License](https://img.shields.io/github/license/FNNDSC/chrs)](https://github.com/FNNDSC/chrs/blob/master/LICENSE)
[![Test](https://github.com/FNNDSC/chrs/actions/workflows/test.yml/badge.svg)](https://github.com/FNNDSC/chrs/actions/workflows/test.yml)
[![Publish](https://github.com/FNNDSC/chrs/actions/workflows/release.yml/badge.svg)](https://github.com/FNNDSC/chrs/actions/workflows/release.yml)

This project provides:

- `chrs`, a command-line client for [_ChRIS_](https://chrisproject.org/)
- `chris`, a [Rust](https://www.rust-lang.org/) client library for _ChRIS_

## Installation

There are two ways to install `chrs`.

### Download

Pre-compiled binaries are automatically built and uploaded to
[Github Releases](https://github.com/FNNDSC/chrs/releases).
Get the latest version here:

https://github.com/FNNDSC/chrs/releases/latest

This is the easiest installation method, however there is no
mechanism for automatic updates.


### Get from Crates.io

Use [cargo](https://doc.rust-lang.org/cargo/) to get and build the
package from crates.io:

```shell
cargo install chrs
```

If necessary, add the `bin` folder to `$PATH`:

```shell
echo 'export PATH=$HOME/.cargo/bin:$PATH' >> ~/.bashrc
source ~/.bashrc
```

## Usage Example

Upload some local files and directories to ChRIS under the path `chrisuser/uploads/my-data`:

```shell
chrs --address http://localhost:8000/api/v1/ \
    --username chrisuser --password chris1234 \
    upload --path "my-data" file1.nii nested/file2.nii folder_of_stuff/
```

Relative path structures are preserved, so the following upload paths will be created:

- `chrisuser/uploads/my-data/file1.nii`
- `chrisuser/uploads/my-data/nested/file2.nii`
- `chrisuser/uploads/my-data/folder_of_stuff/...`

### Fancy Upload

You can use `parallel` to add a progress bar, and in some cases, improve performance.

```shell
find data_dir/ -type f | parallel --bar --eta      \
    'chrs --address http://localhost:8000/api/v1/  \
          --username chris --password chris1234    \
            upload {} > /dev/null'
```

## Known Problems

Relative paths to parent directories, e.g. `../filename`, are not supported.

## Roadmap

See https://github.com/FNNDSC/chrs/projects/1

> Rewrite it in Rust.

`chrs` succeeds [caw](https://github.com/FNNDSC/caw).
Well, I mean there's hope at least.
Right now, the only thing `chrs` can do is upload files.

https://github.com/FNNDSC/chrs/wiki/Feature-Table
