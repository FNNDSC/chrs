# chrs

[![crates.io version](https://img.shields.io/crates/v/chrs?label=version)](https://crates.io/crates/chrs)
[![codecov](https://codecov.io/gh/FNNDSC/chrs/branch/master/graph/badge.svg?flag=chrs&token=UOYL5NPYIP)](https://codecov.io/gh/FNNDSC/chrs)
[![Publish](https://github.com/FNNDSC/chrs/actions/workflows/release.yml/badge.svg)](https://github.com/FNNDSC/chrs/actions/workflows/release.yml)

`chrs` is a command-line client for
[_ChRIS_](https://chrisproject.org).
It can upload files to _ChRIS_ library, download files from _ChRIS_,
and can run and feeds (computational experiments) on _ChRIS_.

## Installation

There are four ways to install `chrs`.

### Direct Download

You can download `chrs` from
[Github Releases](https://github.com/FNNDSC/chrs/releases).
Get the latest version here:

https://github.com/FNNDSC/chrs/releases/latest

This is the easiest installation method, however there is no
mechanism for automatic updates.

### Using Pip

`chrs` is published to [PyPi](https://pypi.org/project/chrs) using
[PyO3/maturin](https://github.com/PyO3/maturin). Installing `chrs`
using `pip` will be preferable for users who are already comfortable with Python.

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

### `chrs switch`

Switch between saved logins.

```shell
# interactive prompt, use arrow keys to choose
chrs switch
  jennings    http://cube-next.tch.harvard.edu/api/v1/
  jenni       http://cube-next.tch.harvard.edu/api/v1/
  rudolph     http://cube-next.tch.harvard.edu/api/v1/
> jennydaman  https://cube.chrisproject.org/api/v1/
  chris       https://cube.chrisproject.org/api/v1/
  
# non-interactive usage
chrs switch --username jennydaman --address https://cube.chrisproject.org/api/v1/
```

### `chrs feeds`

List or search existing feeds.

```shell
$ chrs feeds
COVID-NET analysis on patient ABCD                                       chris/feed_1891
SPL visualization                                                        chris/feed_1890
LLD test 3                                                               chris/feed_1889
Subplate surfaces surface_fit parameter schedule                         jennings/feed_1888
Preprocess data from henry                                               jennings/feed_1887
```

### `chrs ls PATH`

List files and directories in _ChRIS_.

```shell
# by default, folder names appear as feed names or plugin instance titles
$ chrs ls --tree -L 4 chris/feed_1443
chris/Segmented volume data analysis
└── pl-dircopy_5827
    ├── data
    │   ├── output.meta.json
    │   ├── input.meta.json
    │   └── aparc.a2009saseg.mgz
    └── generate volume report
        └── data
            ├── output.meta.json
            ├── mgz2LUT_report.pdf
            ├── mgz2LUT_report.html
            └── input.meta.json

$ chrs ls --tree -L 4 --full --raw chris/feed_1443
chris/feed_1443
└── chris/feed_1443/pl-dircopy_5827
    ├── chris/feed_1443/pl-dircopy_5827/data
    │   ├── chris/feed_1443/pl-dircopy_5827/data/output.meta.json
    │   ├── chris/feed_1443/pl-dircopy_5827/data/input.meta.json
    │   └── chris/feed_1443/pl-dircopy_5827/data/aparc.a2009saseg.mgz
    └── chris/feed_1443/pl-dircopy_5827/pl-mgz2LUT_report_5836
        └── chris/feed_1443/pl-dircopy_5827/pl-mgz2LUT_report_5836/data
            ├── chris/feed_1443/pl-dircopy_5827/pl-mgz2LUT_report_5836/data/output.meta.json
            ├── chris/feed_1443/pl-dircopy_5827/pl-mgz2LUT_report_5836/data/mgz2LUT_report.pdf
            ├── chris/feed_1443/pl-dircopy_5827/pl-mgz2LUT_report_5836/data/mgz2LUT_report.html
            └── chris/feed_1443/pl-dircopy_5827/pl-mgz2LUT_report_5836/data/input.meta.json
```

### `chrs upload FILE...`

Upload files and run workflows.

```shell
# upload some files
chrs upload one_file.txt another_file.txt

# upload all files in a directory
chrs upload my_data/

# upload directory and create a feed with the name "Tractography Study"
chrs upload --feed "Tractography Study" my_data/

# upload directory, create a feed, and run a workflow
chrs upload --feed "Surface Extraction" \
            --pipeline "Fetal Brain Surface Extraction v1.0.0" \
            my_data/
```

### `chrs download SRC [DST]`

Download files from _ChRIS_.

```shell
# download all files created by a feed number 15 into the current directory
chrs download https://cube.chrisproject.org/api/v1/15/files/

# download the output of plugin instance 30, but save the files to a
# directory called "my_outputs" instead of something like
# "<username>/feed_15/pl-dircopy_5550/pl-pfdicom_tagExtract_5551/data/...",
chrs download --shorten https://cube.chrisproject.org/api/v1/plugins/instances/5551/files/ my_outputs

# download files from ChRIS given a path
chrs download chris/uploads/fetal_dataset
chrs download SERVICES/PACS/orthanc/9cfafb0-DIXON_SHANNON_ANON-20140701

# download data from all plugin instances of a feed to the same folder,
# effectively joining their outputs into one directory
chrs download --flatten chris/feed_14 feed14_outputs
```

### `chrs run-latest`

Run a _ChRIS_ plugin (i.e. create a plugin instance) by name.

```shell
# run pl-mri10yr06mo01da_normal (fs-type) to create a new ChRIS feed
chrs run-latest pl-mri10yr06mo01da_normal

# run pl-mri-preview (ds-type) after plugin instance id=56, with option --units-fallback mm
chrs run-latest --memory-limit 2Gi --compute-resource-name moc --previous-id 56 pl-mri-preview  -- --units-fallback mm
```

Note: since plugin version is not specified when using `chrs run-latest`,
the parameters are subject to change. `chrs run` (not yet implemented)
is preferable for the sake of reproducibility.

A plugin's parameters help can be viewed, e.g. for `pl-mri-preview`,
by running `chrs plugin-help pl-mri-preview`

### `chrs get`

Make an authenticated HTTP GET request.

As `chrs` is still under development, many functions are still unavailable.
Advanced users can use `chrs get` to query the CUBE API directly.

```shell
# example: list plugins used in feed 12
cargo run -- get https://cube.chrisproject.org/api/v1/12/plugininstances/ | jq -r '.results[] | .plugin_name'
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

### Feedback

Please report bugs and/or request features here:
https://github.com/FNNDSC/chrs/issues
