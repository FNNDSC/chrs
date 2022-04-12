# chrs

[![crates.io version](https://img.shields.io/crates/v/chrs?label=version)](https://crates.io/crates/chrs)
[![Publish](https://github.com/FNNDSC/chrs/actions/workflows/release.yml/badge.svg)](https://github.com/FNNDSC/chrs/actions/workflows/release.yml)

`chrs` is a command-line client for
[_ChRIS_](https://chrisproject.org).
It can upload files to _ChRIS_ library, download files from _ChRIS_,
and can run and feeds (computational experiments) on _ChRIS_.

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

## Using `chrs`

Run `chrs --help` for usage information.

Note: when specifying URLs, they should be to the backend API, not the front-end.
E.g. instead of `https://app.chrisproject.org/feeds/1520`, the correct URL would
be `https://cube.chrisproject.org/api/v1/1520/`.

### Overview

```shell
chrs --address https://cube.chrisproject.org/api/v1/ --username chris --password chris1234 login
chrs upload my_data/
```

### `chrs login`

`chrs login` saves authentication tokens securely using your
[keyring](https://crates.io/crates/keyring). Logging into multiple
different instances of _ChRIS_ is supported, or as different users
on the same _ChRIS_ instance.

```shell
# log in, type username and password interactively
chrs --address https://cube.chrisproject.org/api/v1/ login

# log in without using keyring and non-interactively, useful for automation
chrs --address https://cube.chrisproject.org/api/v1/ --username test-user login --no-keyring --password-stdin <<< "$PASSWORD"
```

### `chrs logout`

Remove previously saved authentication token(s).

```shell
# remove saved logins for cube.chrisproject.org
chrs --address https://cube.chrisproject.org/api/v1/ logout

# remove all saved logins
chrs logout
```

### `chrs upload FILE...`

Upload files to my _ChRIS_ Library.

```shell
# upload some files
chrs upload one_file.txt another_file.txt

# upload all files in a directory
chrs upload my_data/ 
```

### `chrs pipeline-file add`

Uploads a file-representation of a _ChRIS_ pipeline.
Supported file formats are:

- JSON (`plugin_tree` may be either a string (canonical) or an object)
- [YAML](https://github.com/FNNDSC/CHRIS_docs/blob/master/specs/YAML_Pipelines.adoc)

```shell
chrs pipeline-file add chris/tests/data/pipelines/fetal_brain_reconstruction_expanded.json
```

#### `chrs pipeline-file convert`

Convert between supported pipeline file formats.

````shell
chrs pipeline-file convert pipeline.json pipeline.yml
````
