use std::io::{BufWriter, Write};
use std::path::Path;
use byteorder::{LittleEndian, WriteBytesExt};
use crate::builder::IndexBuilder;
use crate::error::IndexError;
use crate::meta::{self, IndexMeta, INDEX_DIR, INDEX_VERSION};
use crate::types::{LookupEntry, TrigramHash};

/// Write the built index to disk under `root/.trigrep/`.
pub fn write_index(builder: IndexBuilder, root: &Path) -> Result<IndexMeta, IndexError> {
    let index_dir = root.join(INDEX_DIR);
    std::fs::create_dir_all(&index_dir)?;

    // Sort trigrams by hash for binary-searchable lookup table
    let mut sorted: Vec<(TrigramHash, Vec<crate::types::PostingEntry>)> =
        builder.postings.into_iter().collect();
    sorted.sort_by_key(|(hash, _)| *hash);

    // Write postings.bin and collect lookup entries
    let postings_path = index_dir.join("postings.bin");
    let mut postings_writer = BufWriter::new(std::fs::File::create(&postings_path)?);
    let mut lookup_entries: Vec<LookupEntry> = Vec::with_capacity(sorted.len());
    let mut current_offset: u64 = 0;

    for (hash, entries) in &sorted {
        lookup_entries.push(LookupEntry {
            ngram_hash: *hash,
            offset: current_offset,
            length: entries.len() as u32,
        });

        for entry in entries {
            postings_writer.write_u32::<LittleEndian>(entry.file_id)?;
            postings_writer.write_u8(entry.loc_mask)?;
            postings_writer.write_u8(entry.next_mask)?;
            current_offset += 6;
        }
    }
    postings_writer.flush()?;

    // Write lookup.bin
    let lookup_path = index_dir.join("lookup.bin");
    let mut lookup_writer = BufWriter::new(std::fs::File::create(&lookup_path)?);
    for entry in &lookup_entries {
        lookup_writer.write_u32::<LittleEndian>(entry.ngram_hash)?;
        lookup_writer.write_u64::<LittleEndian>(entry.offset)?;
        lookup_writer.write_u32::<LittleEndian>(entry.length)?;
    }
    lookup_writer.flush()?;

    // Write files.bin
    let files_path = index_dir.join("files.bin");
    let mut files_writer = BufWriter::new(std::fs::File::create(&files_path)?);
    for (i, path) in builder.files.iter().enumerate() {
        let path_bytes = path.as_bytes();
        files_writer.write_u32::<LittleEndian>(i as u32)?;
        files_writer.write_u16::<LittleEndian>(path_bytes.len() as u16)?;
        files_writer.write_all(path_bytes)?;
    }
    files_writer.flush()?;

    // Calculate total index size
    let index_size = std::fs::metadata(&postings_path)?.len()
        + std::fs::metadata(&lookup_path)?.len()
        + std::fs::metadata(&files_path)?.len();

    // Write meta.json
    let meta = IndexMeta {
        version: INDEX_VERSION,
        created_at: chrono_now(),
        repo_root: root.canonicalize()?.to_string_lossy().into_owned(),
        num_files: builder.files.len() as u32,
        num_trigrams: lookup_entries.len() as u32,
        index_size_bytes: index_size,
        git_head: meta::git_head(root),
    };
    meta.write(&index_dir)?;

    Ok(meta)
}

fn chrono_now() -> String {
    // Simple ISO 8601 timestamp without chrono dependency
    let output = std::process::Command::new("date")
        .arg("-u")
        .arg("+%Y-%m-%dT%H:%M:%SZ")
        .output();
    match output {
        Ok(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout).trim().to_string()
        }
        _ => "unknown".to_string(),
    }
}
