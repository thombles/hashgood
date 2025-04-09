# hashgood

A CLI tool for easily verifying a downloaded file's checksum.

Compare a file with an **MD5**, **SHA1**, **SHA256** or **SHA512** hash:

* Passed as a command line argument
* SHASUMS-style check files (`-c`)
* Raw hash in a file/STDIN (`-c`)

...or just run hashgood against the input and receive all four at once.

![Image](https://github.com/user-attachments/assets/ffb80986-751d-4b45-9ced-fef0e6bbfba6)

This program arose from dissatisfaction with the <a href="https://thomask.sdf.org/blog/2019/05/05/techniques-for-verifying-shasums-conveniently.html">workarounds required for traditional tools</a>.

## Installation

The easiest and recommended way to install hashgood is by downloading the appropriate package for your platform from the releases section on GitHub.

If you are a developer with a Rust toolchain you can install hashgood directly from crates.io:

```
cargo install hashgood
```

## Usage

Hashgood operates on a single file at a time. It has two main modes:

1. Provide an input file, and hashgood will calculate all the hash types simultaneously.
2. Provide an input file and a hash, and hashgood will calculate the file's actual hash of the same type and check whether it matches the hash you provided.

When you are passing a filename to hashgood, which could be either the input file or a checksum file, you may use the special name `-` (single hyphen) to read it from standard input.

You can get a help summary using the `--help` flag.

```
$ hashgood --help
hashgood 0.5.0

USAGE:
    hashgood [FLAGS] [OPTIONS] <input> [hash]

FLAGS:
    -h, --help         Prints help information
    -C, --no-colour    Disable ANSI colours in output
    -V, --version      Prints version information

OPTIONS:
    -c, --check <hash-file>    A file containing the hash to verify.
                               It can either be a raw hash or a
                               SHASUMS-style listing. Use `-` for
                               standard input

ARGS:
    <input>    The file to be verified or `-` for standard input
    <hash>     A hash to verify, supplied directly on the command line
```

### Calculate All Hashes

To calculate all hash types, pass the path of the file: `hashgood FILENAME`

There is no way to get undecorated output or request a specific hash. If you want to write a script that needs a particular type then you should use a tool designed for computer-readable output.

![Image](https://github.com/user-attachments/assets/3ece0ca7-4aa2-495b-844a-8f66059d7b45)

![Image](https://github.com/user-attachments/assets/6781b31a-7659-4c55-a287-27fda32a695f)

### Verify a Hash Directly

The easiest way to check a hash is to pass it in on the command line: `hashgood FILENAME HASH`

Hashgood will detect what type of hash it is based on its length, calculate it by reading the input file, then show you whether or not it matched. An example screenshot is shown at the top of this page.

### Use a SHASUMS File

Many projects will create files with names like `SHASUMS` or `SHA512SUMS`. These digests have a slightly peculiar format but the basic idea is that it aggregates one or more checksums into a file, listing the files and and their corresponding hashes. You might download an ISO file and the `SHA256SUMS` file that is in the same directory. You can check it with the `-c` option: `hashgood -c SHA256SUMS FILE`

![Image](https://github.com/user-attachments/assets/137209f3-03f5-4ed5-95e1-36eb82e2335d)

Some things to be aware of:

* Hashgood will tolerate the file passed via `-c` not being in the peculiar SHASUMS format; so long it contains a valid hex hash then it will be used.
* Hashgood does not support using `-c` to check _all_ files listed in the checksum file. The input file must always be specified and you can only verify one at a time.
* If the input filename does not match what's listed in the SHASUMS but the hash matches, hashgood will indicate a "MAYBE" result with a warning about the mismatch. This also happens if you provide the input data via STDIN, since it is impossible to know what the original filename is.

## Project Goals

* Be forgiving and deliver what the user wants with a minimum of fuss. They just want to check this hash, damnit.
* Don't let users be tricked—be explicit about checksum types and the sources of those checksums that are being compared.
* As much cross-platform support as is practical.

## Project Non-Goals

* Scriptability. This is an interactive tool.
* Support for any unusual scenarios that could compromise smooth operation. (e.g., text mode, uncommon hash types)

## Future Ideas

* Nominate a default (downloads) directory and auto-select the most recently created file in that directory as input.
* Support bulk checking of all files listed in a checksum file.
