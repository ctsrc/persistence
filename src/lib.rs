/*
 * Copyright (c) 2019 Erik Nordstrøm <erik@nordstroem.no>
 *
 * Permission to use, copy, modify, and/or distribute this software for any
 * purpose with or without fee is hereby granted, provided that the above
 * copyright notice and this permission notice appear in all copies.
 *
 * THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
 * WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
 * MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
 * ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
 * WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
 * ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
 * OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
 */

//!
//! # persistence – Rust library for mutable resizable arrays built on top of mmap
//!
//! A resizable, mutable array type implemented in Rust on top of mmap,
//! providing persistence for arrays of data in memory.
//!
//! ## Advisory locking
//!
//! POSIX advisory locks are not without problems. See for example
//! [this comment in the source code of SQLite](https://www.sqlite.org/src/artifact/c230a7a24?ln=994-1081)
//! for a good write-up about the kinds of problems the developers of SQLite
//! see with POSIX advisory locks, and some pitfalls that one should be aware of.
//!
//! The persistence library is aimed at a particular group of use-cases where
//! POSIX advisory locks happen to be suitable. It is imperative then, that would-be users
//! of the persistence library are aware of what that group of use-cases is. In the section
//! that follows, we write a bit about just that; who and what this library is for.
//!
//! ## Who and what this library is for
//!
//! This library is aimed at developers who wish to write software utilizing
//! [data-oriented design](https://en.wikipedia.org/wiki/Data-oriented_design)
//! techniques in run-time environments where all of the following hold true:
//!
//!   1. You require that the data in question be persisted to disk, and
//!   2. You require that the data in question be synced to disk at certain times
//!      or intervals, after said data has been mutated (added to, deleted from, or altered),
//!      such that abnormal termination of your program (e.g. program crash, loss of power, etc.)
//!      incurs minimal loss of data, and
//!   3. You are confident that all processes which rely on the data on disk honor the
//!      POSIX advisory locks that we apply to them, so that the integrity of the data is ensured.
//!
//! ## Motivation
//!
//! Data persistence is achievable by many different means. No one solution fits all
//! (and this library is no exception from that).
//!
//! Some of the ways in which data persistence can be achieved include:
//!
//!   - Relying on a relational database such as [PostgreSQL](https://www.postgresql.org).
//!   - Making use of the [Serde](https://serde.rs) framework for serializing and deserializing
//!     Rust data structures, and handle writing to and reading from disk yourself.
//!
//! But, when you are architecting software, and you choose to apply the data-oriented design
//! paradigm to your problem, you may find that you end up with some big arrays of data where
//! you've ordered the elements of each array in such a way as to be optimized for
//! [CPU caches](https://en.wikipedia.org/wiki/CPU_cache) in terms of
//! [spatial locality of reference](https://en.wikipedia.org/wiki/Locality_of_reference#Types_of_locality).
//!
//! **When that is the case** – when you have those kinds of arrays, and when you want to persist
//! the data in those arrays in the manner we talked about [further up in this document](#who-and-what-this-library-is-for),
//! `mmap()`'ing those arrays to files on disk begins to look *pretty* alluring,
//! doesn't it? And there you have it, that was the motivation for writing this library.
//!
//! ## What this library is, and what it is not
//!
//! This library helps you out when you have arrays of data that are being mutated at run-time,
//! and you need to sync the data to disk for persistence at certain points or intervals in time.
//! It does so by making use of `mmap()` (through [the `memmap` crate](https://crates.io/crates/memmap))
//! with a little bit of locking and data validation sprinkled on top.
//!
//! What this library is **not** is, *something that "gives you" data-oriented design*. Indeed,
//! there can be no such thing;
//!
//! <blockquote>
//!   A big misunderstanding for many new to the data-oriented design paradigm, a concept brought
//!   over from abstraction based development, is that we can design a static library or set of
//!   templates to provide generic solutions to everything presented in this book as a
//!   data-oriented solution. Much like with domain driven design, data-oriented design is product
//!   and work-flow specific. You learn how to do data-oriented design, not how to add it to your
//!   project. The fundamental truth is that data, though it can be generic by type,
//!   is not generic in how it is used.
//!
//!   <footer>
//!     — <cite>
//!         Richard Fabian,
//!         <a href=http://www.dataorienteddesign.com/dodbook/node2.html#SECTION00240000000000000000>Data-Oriented Design. Chapter 1, sub-section "Data can change".</a>
//!       </cite>
//!   </footer>
//! </blockquote>
//!
//! ## Caveats or, some things to keep in mind
//!
//! This library makes use of POSIX advisory locks on Unix platforms.
//!
//! TODO: Write about how to use the library correctly.
//!
//! ## READY? LET'S GO!
//!
//! Add [the persistence crate](https://crates.io/crates/persistence) to the `[dependencies]`
//! section of [your `Cargo.toml` manifest](https://doc.rust-lang.org/cargo/reference/manifest.html)
//! and start using this library in your projects.
//!
//! ## Star me on GitHub
//!
//! Don't forget to star [persistence on GitHub](https://github.com/ctsrc/persistence)
//! if you find this library interesting or useful.
//!

use memmap::MmapMut;
use std::{io, slice};
use std::fs::{OpenOptions, File};
use std::path::{Path, PathBuf};
use std::mem;
use std::io::Write;
use fs2::FileExt;

#[cfg(test)]
use tempfile::TempDir;

#[repr(C, packed)]
pub struct FileHeader<T>
{
  magic_bytes: [u8; 8],
  endianness: u16,
  persistence_version: [u8; 3],
  data_contained_version: [u8; 3],
  default_data: T,
}

pub struct Persistent
{
  file: File,
  mm: MmapMut,
}

impl Persistent
{
  fn new<T: Sized + Default> (path: &Path, magic_bytes: [u8; 8], data_contained_version: [u8; 3]) -> io::Result<Self>
  {
    let mut file = OpenOptions::new().read(true).write(true).create(true).open(path)?;

    /*
     * NOTE: The fs2 library is cross-platform beyond just the platforms that we support.
     *       We use this library not because we want to try and support all of those,
     *       but because it covers what we want to do and saves us some typing and thinking.
     *       See the section about advisory locking the doc comments of this file.
     */
    file.try_lock_exclusive()?;

    let fhs = mem::size_of::<FileHeader<T>>();

    let fh = FileHeader
    {
      magic_bytes,
      endianness: 0x1234,
      persistence_version: [0, 1, 0],
      data_contained_version,
      default_data: T::default(),
    };

    if file.metadata().unwrap().len() == 0
    {
      let buf = unsafe
      {
        slice::from_raw_parts(
          &fh as *const FileHeader<T> as *const u8,
          mem::size_of::<FileHeader<T>>())
      };
      file.write(buf)?;
    }
    else
    {
      // TODO: Validate header.
    }

    let mut mm = unsafe { MmapMut::map_mut(&file)? };

    Ok(Self
    {
      file,
      mm,
    })
  }
}

/// Helper function for tests.
#[cfg(test)]
fn persist_to_tempfile () -> io::Result<(TempDir, PathBuf, Persistent)>
{
  #[repr(C, packed)]
  struct Example
  {
    hello: u8,
    world: u8,
  }

  impl Default for Example
  {
    fn default () -> Self
    {
      Self
      {
        hello: 1,
        world: 2,
      }
    }
  }

  let dir = tempfile::tempdir()?;

  let pathbuf = dir.path().join("file.bin");
  let path = pathbuf.as_path();

  let magic_bytes = [b'T', b'E', b'S', b'T', b'F', b'I', b'L', b'E'];
  let data_contained_version = [0, 1, 0];

  let p = Persistent::new::<Example>(path, magic_bytes, data_contained_version)?;

  Ok((dir, pathbuf, p))
}

#[test]
pub fn test_create () -> Result<(), io::Error>
{
  persist_to_tempfile()?;

  Ok(())
}

#[test]
pub fn test_file_is_locked () -> Result<(), io::Error>
{
  let (dir, pathbuf, p) = persist_to_tempfile()?;

  // TODO: Spawn process which tries to lock file.

  unimplemented!()
}

#[test]
pub fn test_file_is_unlocked_after_drop () -> Result<(), io::Error>
{
  let (dir, pathbuf, _) = persist_to_tempfile()?;

  // TODO: Spawn process which tries to lock file.

  unimplemented!()
}

#[test]
pub fn test_header_corrupt_magic_bytes () -> Result<(), io::Error>
{
  unimplemented!()
}

#[test]
pub fn test_file_corrupt_truncated_to_under_end_of_header () -> Result<(), io::Error>
{
  unimplemented!()
}
