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
//! # persistence – mutable resizable arrays built on top of mmap
//!
//! This Rust library provides [`MmapedVec`](MmapedVec); a resizable, mutable array type
//! implemented on top of [`mmap()`](https://pubs.opengroup.org/onlinepubs/7908799/xsh/mmap.html),
//! providing a [`Vec`](https://doc.rust-lang.org/std/vec/struct.Vec.html)-like data structure
//! with persistence to disk built into it.
//!
//! [`MmapedVec`](MmapedVec) is aimed at developers who wish to write software utilizing
//! [data-oriented design](https://en.wikipedia.org/wiki/Data-oriented_design)
//! techniques in run-time environments where all of the following hold true:
//!
//!   1. You have determined that a `Vec`-like data structure is appropriate for some
//!      or all of your data, and
//!   2. You require that the data in question be persisted to disk, and
//!   3. You require that the data in question be synced to disk at certain times
//!      or intervals, after said data has been mutated (added to, deleted from, or altered),
//!      such that abnormal termination of your program (e.g. program crash, loss of power, etc.)
//!      incurs minimal loss of data, and
//!   4. You are confident that all processes which rely on the data on disk honor the
//!      POSIX advisory locks that we apply to them, so that the integrity of the data is
//!      ensured, and
//!   5. You desire, or at least are fine with, having the on-disk representation of your data
//!      be the same as that which it has in memory, and understand that this means that the files
//!      are tied to the CPU architecture of the host that they were saved to disk on. If you need
//!      to migrate your data to another computer with a different CPU architecture in the future,
//!      you convert it then, rather than serializing and deserializing your data between some
//!      other format and the in-memory representation all of the time.
//!
//! ## POSIX advisory locks
//!
//! This library makes use of POSIX advisory locks on Unix platforms. POSIX advisory locks are
//! not without problems. See for example
//! [this comment in the source code of SQLite](https://www.sqlite.org/src/artifact/c230a7a24?ln=994-1081)
//! for a good write-up about the kinds of problems the developers of SQLite
//! see with POSIX advisory locks, and some pitfalls that one should be aware of.
//!
//! Provided that your software runs in an environment where any process that attempts to open
//! the files you are persisting your data to honor the advisory locks, and you take care not
//! to open the same file multiple times within the same process (as per what was said in the
//! comment linked above), everything will be fine and dandy :)
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
//! But, in software architecture situations where you choose to apply the data-oriented design
//! paradigm to your problem, you may find that you end up with some big arrays of data where
//! you've ordered the elements of each array in such a way as to be optimized for
//! [CPU caches](https://en.wikipedia.org/wiki/CPU_cache) in terms of
//! [spatial locality of reference](https://en.wikipedia.org/wiki/Locality_of_reference#Types_of_locality).
//!
//! **When that is the case** – when you have those kinds of arrays, and when you want to persist
//! the data in those arrays in the manner we talked about
//! [at the beginning of this document](#persistence--mutable-resizable-arrays-built-on-top-of-mmap),
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

use std::marker::PhantomData;
use std::{io, slice};
use std::fs::{OpenOptions, File};
use std::path::Path;
use std::mem;
use std::io::Write;
use memmap::MmapMut;
use fs2::FileExt;

#[repr(C, packed)]
pub struct FileHeader<T>
{
  magic_bytes: [u8; 8],
  endianness: u16,
  persistence_version: [u8; 3],
  data_contained_version: [u8; 3],
  default_data: T,
}

pub struct MmapedVec<T>
{
  file: File,
  mm: MmapMut,
  _marker: PhantomData<T>,
}

impl<T: Sized + Default> MmapedVec<T>
{
  fn try_new (path: &Path, magic_bytes: [u8; 8], data_contained_version: [u8; 3]) -> io::Result<Self>
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
      _marker: PhantomData,
    })
  }
}

#[cfg(test)]
mod tests
{
  use super::*;
  use std::path::PathBuf;
  use std::process::{Command, ExitStatus, Stdio};
  use tempfile::TempDir;

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

  /// Helper function for tests.
  fn new_mmaped_vec_of_example_persisting_in_tempdir () -> io::Result<(TempDir, PathBuf, MmapedVec<Example>)>
  {
    let dir = tempfile::tempdir()?;

    let pathbuf = dir.path().join("file.bin");
    let path = pathbuf.as_path();

    let magic_bytes = [b'T', b'E', b'S', b'T', b'F', b'I', b'L', b'E'];
    let data_contained_version = [0, 1, 0];

    let p = MmapedVec::try_new(path, magic_bytes, data_contained_version)?;

    Ok((dir, pathbuf, p))
  }

  /// Helper function for tests.
  fn python3_try_lock_exclusive (path: &Path) -> io::Result<ExitStatus>
  {
    // NOTE: Keep in mind that if the parent test fails, python3 might not be in your $PATH.

    let mut child = Command::new("python3").arg("-").arg(path)
      .stdin(Stdio::piped()).stdout(Stdio::inherit()).stderr(Stdio::inherit())
      .spawn()?;

    let child_stdin = child.stdin.as_mut().unwrap();
    child_stdin.write_all(include_bytes!("../scripts/try_lock_exclusive.py"))?;

    child.wait()
  }

  #[test]
  pub fn test_create_mmaped_vec_onto_tempfile () -> Result<(), io::Error>
  {
    new_mmaped_vec_of_example_persisting_in_tempdir()?;

    Ok(())
  }

  #[test]
  pub fn test_file_is_locked_while_fd_is_held () -> Result<(), io::Error>
  {
    let (dir, pathbuf, p) = new_mmaped_vec_of_example_persisting_in_tempdir()?;

    assert_eq!(python3_try_lock_exclusive(pathbuf.as_path())?.code(), Some(35));

    /*
     * XXX: We run the script an additional time to be sure that the test itself does not
     *      duplicate and close the fd for the file in a way that results in release of the
     *      advisory lock.
     */
    assert_eq!(python3_try_lock_exclusive(pathbuf.as_path())?.code(), Some(35));

    Ok(())
  }

  #[test]
  pub fn test_file_is_unlocked_after_drop () -> Result<(), io::Error>
  {
    let (dir, pathbuf, _) = new_mmaped_vec_of_example_persisting_in_tempdir()?;

    assert_eq!(python3_try_lock_exclusive(pathbuf.as_path())?.code(), Some(0));

    Ok(())
  }

  #[test]
  pub fn test_detect_header_corrupt_magic_bytes () -> Result<(), io::Error>
  {
    unimplemented!()
  }

  #[test]
  pub fn test_detect_file_corrupt_truncated_to_under_end_of_header () -> Result<(), io::Error>
  {
    unimplemented!()
  }
}
