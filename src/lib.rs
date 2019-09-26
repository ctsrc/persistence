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
//! ## Advisory locking
//!
//! POSIX advisory locks are not without problems. See for example
//! [this comment in the source code of SQLite](https://www.sqlite.org/src/artifact/c230a7a24?ln=994-1081)
//! for a good write-up about the kinds of problems the developers
//! of SQLite see with POSIX advisory locks, and some pitfalls that
//! one should be aware of.
//!
//! The persistence library is aimed at a particular group of use-cases
//! where POSIX advisory locks happen to be suitable. It is imperative
//! then, that would-be users of the persistence library are aware of
//! what that group of use-cases is. In the section that follows, we
//! write a bit about just that; who and what this library is for.
//!
//! ## Who and what this library is for
//!
//! TODO: Write about who and what the library is for.
//!
//! ## Caveats or, some things to keep in mind
//!
//! This library makes use of POSIX advisory locks on Unix platforms.
//!
//! TODO: Write about how to use the library correctly.
//!

use memmap::MmapMut;
use std::{io, slice};
use std::fs::{OpenOptions, File};
use std::path::{Path, PathBuf};
use std::mem;
use std::io::Write;
use fs2::FileExt;
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
