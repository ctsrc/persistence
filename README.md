# persistence – mutable resizable arrays built on top of mmap

[![Crates.io](https://img.shields.io/crates/v/persistence.svg)](https://crates.io/crates/persistence) [![Docs.rs](https://docs.rs/persistence/badge.svg)](https://docs.rs/persistence/)

This Rust library provides [`MmapedVec`](https://docs.rs/persistence/0.0.3/persistence/struct.MmapedVec.html);
a resizable, mutable array type implemented on top of
[`mmap()`](https://pubs.opengroup.org/onlinepubs/7908799/xsh/mmap.html),
providing a [`Vec`](https://doc.rust-lang.org/std/vec/struct.Vec.html)-like data structure
with persistence to disk built into it.

`MmapedVec` is aimed at developers who wish to write software utilizing
[data-oriented design](https://en.wikipedia.org/wiki/Data-oriented_design)
techniques in run-time environments where all of the following hold true:

  1. You have determined that a `Vec`-like data structure is appropriate for some
     or all of your data, and
  2. You require that the data in question be persisted to disk, and
  3. You require that the data in question be synced to disk at certain times
     or intervals, after said data has been mutated (added to, deleted from, or altered),
     such that abnormal termination of your program (e.g. program crash, loss of power, etc.)
     incurs minimal loss of data, and
  4. You are confident that all processes which rely on the data on disk honor the
     POSIX advisory locks that we apply to them, so that the integrity of the data is
     ensured, and
  5. You desire, or at least are fine with, having the on-disk representation of your data
     be the same as that which it has in memory, and understand that this means that the files
     are tied to the CPU architecture of the host that they were saved to disk on. If you need
     to migrate your data to another computer with a different CPU architecture in the future,
     you convert it then, rather than serializing and deserializing your data between some
     other format and the in-memory representation all of the time.

## POSIX advisory locks

This library makes use of POSIX advisory locks on Unix platforms. POSIX advisory locks are
not without problems. See for example
[this comment in the source code of SQLite](https://www.sqlite.org/src/artifact/c230a7a24?ln=994-1081)
for a good write-up about the kinds of problems the developers of SQLite
see with POSIX advisory locks, and some pitfalls that one should be aware of.

Provided that your software runs in an environment where any process that attempts to open
the files you are persisting your data to honor the advisory locks, and you take care not
to open the same file multiple times within the same process (as per what was said in the
comment linked above), everything will be fine and dandy :)

## Learn more and get started

[Read the docs](https://docs.rs/persistence/) to learn more
about what this library is for and how you use it.

## Star me on GitHub

Don't forget to star [persistence on GitHub](https://github.com/ctsrc/persistence)
if you find this library interesting or useful.
