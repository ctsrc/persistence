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

use memmap::MmapMut;
use std::{io, slice};
use std::fs::OpenOptions;
use std::path::Path;
use std::mem;
use std::io::Write;

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
  mm: MmapMut,
}

impl Persistent
{
  fn new<T: Sized + Default> (path: &Path, magic_bytes: [u8; 8], data_contained_version: [u8; 3]) -> io::Result<Self>
  {
    let mut file = OpenOptions::new().read(true).write(true).create(true).open(path)?;

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
      mm
    })
  }
}

/// Helper function for tests.
fn tempfile_persistent () -> io::Result<Persistent>
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

  Persistent::new::<Example>(path, magic_bytes, data_contained_version)
}

#[test]
pub fn test_create () -> Result<(), io::Error>
{
  tempfile_persistent()?;

  Ok(())
}

#[test]
pub fn test_header_corrupt_magic_bytes () -> Result<(), io::Error>
{
  // TODO: Corrupt and check each magic byte individually
  unimplemented!()
}

#[test]
pub fn test_file_corrupt_truncated_to_under_end_of_header () -> Result<(), io::Error>
{
  unimplemented!()
}

// TODO: Trunc to under default data
