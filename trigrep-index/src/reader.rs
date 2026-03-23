use std::cmp::Ordering;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use byteorder::{LittleEndian, ReadBytesExt};
use memmap2::Mmap;
use crate::error::IndexError;
use crate::meta::{IndexMeta, INDEX_DIR};
use crate::types::{PostingEntry, TrigramHash};

/// Memory-mapped index reader for fast trigram lookups.
pub struct IndexReader {
    lookup_mmap: Mmap,
    postings_file: File,
    pub files: Vec<String>,
    pub meta: IndexMeta,
    num_lookup_entries: usize,
}

impl IndexReader {
    /// Open an existing index from `root/.trigrep/`.
    pub fn open(root: &Path) -> Result<Self, IndexError> {
        let index_dir = root.join(INDEX_DIR);
        let meta = IndexMeta::read(&index_dir)?;

        // mmap lookup.bin
        let lookup_file = File::open(index_dir.join("lookup.bin"))?;
        let lookup_mmap = unsafe { Mmap::map(&lookup_file)? };
        let num_lookup_entries = lookup_mmap.len() / 16;

        // Open postings.bin for random reads
        let postings_file = File::open(index_dir.join("postings.bin"))?;

        // Load files.bin into memory
        let files = read_files_bin(&index_dir.join("files.bin"))?;

        Ok(Self {
            lookup_mmap,
            postings_file,
            files,
            meta,
            num_lookup_entries,
        })
    }

    pub fn num_files(&self) -> usize {
        self.files.len()
    }

    /// Binary search the lookup table for a trigram hash.
    /// Returns (byte_offset_in_postings, num_entries) if found.
    pub fn lookup(&self, hash: TrigramHash) -> Option<(u64, u32)> {
        let data = &self.lookup_mmap[..];
        let n = self.num_lookup_entries;
        let mut lo = 0usize;
        let mut hi = n;

        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let base = mid * 16;
            let entry_hash = u32::from_le_bytes(data[base..base + 4].try_into().unwrap());

            match entry_hash.cmp(&hash) {
                Ordering::Less => lo = mid + 1,
                Ordering::Greater => hi = mid,
                Ordering::Equal => {
                    let offset =
                        u64::from_le_bytes(data[base + 4..base + 12].try_into().unwrap());
                    let length =
                        u32::from_le_bytes(data[base + 12..base + 16].try_into().unwrap());
                    return Some((offset, length));
                }
            }
        }
        None
    }

    /// Read a posting list from the postings file.
    pub fn read_posting_list(&mut self, hash: TrigramHash) -> Result<Vec<PostingEntry>, IndexError> {
        let (offset, length) = match self.lookup(hash) {
            Some(v) => v,
            None => return Ok(Vec::new()),
        };

        self.postings_file.seek(SeekFrom::Start(offset))?;
        let mut entries = Vec::with_capacity(length as usize);

        for _ in 0..length {
            let file_id = self.postings_file.read_u32::<LittleEndian>()?;
            let loc_mask = self.postings_file.read_u8()?;
            let next_mask = self.postings_file.read_u8()?;
            entries.push(PostingEntry {
                file_id,
                loc_mask,
                next_mask,
            });
        }

        Ok(entries)
    }

    /// Get the relative path for a file ID.
    pub fn file_path(&self, file_id: u32) -> &str {
        &self.files[file_id as usize]
    }
}

/// Read the files.bin table into a Vec<String>.
fn read_files_bin(path: &Path) -> Result<Vec<String>, IndexError> {
    let mut file = File::open(path)?;
    let file_len = file.metadata()?.len();
    let mut files = Vec::new();
    let mut pos = 0u64;

    while pos < file_len {
        let _file_id = file.read_u32::<LittleEndian>()?;
        let path_len = file.read_u16::<LittleEndian>()? as usize;
        let mut path_bytes = vec![0u8; path_len];
        file.read_exact(&mut path_bytes)?;
        let path_str = String::from_utf8_lossy(&path_bytes).into_owned();
        files.push(path_str);
        pos += 4 + 2 + path_len as u64;
    }

    Ok(files)
}
