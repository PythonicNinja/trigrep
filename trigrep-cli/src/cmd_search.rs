use std::path::Path;
use std::time::Instant;
use anyhow::{Context, Result};
use rayon::prelude::*;
use regex::Regex;
use trigrep_index::IndexReader;
use trigrep_index::query;

use crate::output::{OutputConfig, SearchMatch, print_matches};
use crate::regex_decompose;

pub struct SearchOptions {
    pub pattern: String,
    pub case_insensitive: bool,
    pub count_only: bool,
    pub files_only: bool,
    pub line_numbers: bool,
    pub word_boundary: bool,
    pub json: bool,
    pub no_index: bool,
    pub stats: bool,
    pub context_before: usize,
    pub context_after: usize,
}

pub fn run(opts: &SearchOptions, path: &Path) -> Result<()> {
    let root = path.canonicalize()?;
    let start = Instant::now();

    // Build the regex pattern
    let mut regex_pattern = opts.pattern.clone();
    if opts.word_boundary {
        regex_pattern = format!(r"\b{}\b", regex_pattern);
    }
    let regex = regex::RegexBuilder::new(&regex_pattern)
        .case_insensitive(opts.case_insensitive)
        .build()
        .context("Invalid regex pattern")?;

    let matches = if opts.no_index {
        search_brute_force(&regex, &root)?
    } else {
        search_indexed(opts, &regex, &root)?
    };

    let elapsed = start.elapsed();

    let config = OutputConfig {
        json: opts.json,
        count_only: opts.count_only,
        files_only: opts.files_only,
        line_numbers: opts.line_numbers,
        color: !opts.json && atty::is(atty::Stream::Stdout),
        context_before: opts.context_before,
        context_after: opts.context_after,
    };

    print_matches(&matches, &config)?;

    if opts.stats {
        eprintln!(
            "[stats] {} matches in {:.3}s",
            matches.len(),
            elapsed.as_secs_f64()
        );
    }

    Ok(())
}

fn search_indexed(opts: &SearchOptions, regex: &Regex, root: &Path) -> Result<Vec<SearchMatch>> {
    // Try to open index, auto-build if missing
    let mut reader = match IndexReader::open(root) {
        Ok(r) => r,
        Err(trigrep_index::IndexError::NotFound { .. }) => {
            eprintln!("No index found. Building index...");
            crate::cmd_index::run(root)?;
            IndexReader::open(root)?
        }
        Err(e) => return Err(e.into()),
    };

    // Decompose regex into query plan
    let plan = regex_decompose::decompose(&opts.pattern, opts.case_insensitive);

    if opts.stats {
        eprintln!("[stats] Query plan: {:?}", plan);
    }

    // Execute query plan to get candidate file IDs
    let candidates = query::execute(&mut reader, &plan)?;

    if opts.stats {
        eprintln!(
            "[stats] {} candidate files out of {} total",
            candidates.len(),
            reader.num_files()
        );
    }

    // Run regex on candidate files in parallel
    let mut matches: Vec<SearchMatch> = candidates
        .par_iter()
        .flat_map(|&file_id| {
            let rel_path = reader.file_path(file_id).to_string();
            let abs_path = root.join(&rel_path);
            match std::fs::read_to_string(&abs_path) {
                Ok(content) => search_file_content(&rel_path, &content, regex),
                Err(_) => Vec::new(),
            }
        })
        .collect();

    // Path-name matches: regex on every indexed file path. Files that already
    // have content matches are skipped (no duplicate path-only line).
    let content_hits: std::collections::HashSet<&str> =
        matches.iter().map(|m| m.file.as_str()).collect();
    let path_matches: Vec<SearchMatch> = reader
        .files
        .par_iter()
        .filter_map(|rel_path| {
            if content_hits.contains(rel_path.as_str()) {
                return None;
            }
            if regex.is_match(rel_path) {
                Some(SearchMatch {
                    file: rel_path.clone(),
                    line_number: 0,
                    line_content: String::new(),
                })
            } else {
                None
            }
        })
        .collect();
    matches.extend(path_matches);

    Ok(matches)
}

fn search_brute_force(regex: &Regex, root: &Path) -> Result<Vec<SearchMatch>> {
    let entries = trigrep_index::walker::walk_files(root)?;

    let matches: Vec<SearchMatch> = entries
        .par_iter()
        .flat_map(|entry| {
            let content = String::from_utf8_lossy(&entry.content);
            let mut hits = search_file_content(&entry.relative_path, &content, regex);
            if hits.is_empty() && regex.is_match(&entry.relative_path) {
                hits.push(SearchMatch {
                    file: entry.relative_path.clone(),
                    line_number: 0,
                    line_content: String::new(),
                });
            }
            hits
        })
        .collect();

    Ok(matches)
}

fn search_file_content(rel_path: &str, content: &str, regex: &Regex) -> Vec<SearchMatch> {
    content
        .lines()
        .enumerate()
        .filter_map(|(line_no, line)| {
            if regex.is_match(line) {
                Some(SearchMatch {
                    file: rel_path.to_string(),
                    line_number: line_no + 1,
                    line_content: line.to_string(),
                })
            } else {
                None
            }
        })
        .collect()
}
