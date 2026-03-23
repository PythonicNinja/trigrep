use std::collections::HashMap;
use std::path::Path;
use rayon::prelude::*;
use crate::error::IndexError;
use crate::trigram::extract_trigrams;
use crate::types::{PostingEntry, TrigramHash};
use crate::walker;

/// In-memory index builder that accumulates trigram posting data.
pub struct IndexBuilder {
    pub files: Vec<String>,
    pub postings: HashMap<TrigramHash, Vec<PostingEntry>>,
}

impl IndexBuilder {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            postings: HashMap::new(),
        }
    }

    /// Walk and index all text files under the given root directory.
    pub fn add_directory(&mut self, root: &Path) -> Result<(), IndexError> {
        let entries = walker::walk_files(root)?;

        // Extract trigrams per file in parallel
        let per_file: Vec<(String, HashMap<TrigramHash, (u8, u8)>)> = entries
            .into_par_iter()
            .map(|entry| {
                let trigrams = extract_trigrams(&entry.content);
                (entry.relative_path, trigrams)
            })
            .collect();

        // Merge into global index (sequential — modifies shared state)
        for (rel_path, file_trigrams) in per_file {
            let file_id = self.files.len() as u32;
            self.files.push(rel_path);

            for (hash, (loc_mask, next_mask)) in file_trigrams {
                self.postings.entry(hash).or_default().push(PostingEntry {
                    file_id,
                    loc_mask,
                    next_mask,
                });
            }
        }

        Ok(())
    }

    pub fn num_files(&self) -> u32 {
        self.files.len() as u32
    }

    pub fn num_trigrams(&self) -> u32 {
        self.postings.len() as u32
    }
}
