use colored::Colorize;
use std::io::{self, Write};

/// A single search match.
#[derive(Debug, Clone)]
pub struct SearchMatch {
    pub file: String,
    pub line_number: usize,
    pub line_content: String,
}

/// Output format options.
#[allow(dead_code)]
pub struct OutputConfig {
    pub json: bool,
    pub count_only: bool,
    pub files_only: bool,
    pub line_numbers: bool,
    pub color: bool,
    pub context_before: usize,
    pub context_after: usize,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            json: false,
            count_only: false,
            files_only: false,
            line_numbers: true,
            color: atty::is(atty::Stream::Stdout),
            context_before: 0,
            context_after: 0,
        }
    }
}

/// Print matches according to the output configuration.
pub fn print_matches(matches: &[SearchMatch], config: &OutputConfig) -> io::Result<()> {
    let stdout = io::stdout();
    let mut out = stdout.lock();

    if config.json {
        print_json(matches, &mut out)?;
    } else if config.files_only {
        print_files_only(matches, &mut out, config.color)?;
    } else if config.count_only {
        print_count(matches, &mut out, config.color)?;
    } else {
        print_grep_format(matches, &mut out, config)?;
    }

    Ok(())
}

fn print_json(matches: &[SearchMatch], out: &mut impl Write) -> io::Result<()> {
    for m in matches {
        writeln!(
            out,
            r#"{{"file":"{}","line":{},"content":"{}"}}"#,
            m.file.replace('\\', "\\\\").replace('"', "\\\""),
            m.line_number,
            m.line_content.replace('\\', "\\\\").replace('"', "\\\""),
        )?;
    }
    Ok(())
}

fn print_files_only(
    matches: &[SearchMatch],
    out: &mut impl Write,
    color: bool,
) -> io::Result<()> {
    let mut seen = std::collections::HashSet::new();
    for m in matches {
        if seen.insert(&m.file) {
            if color {
                writeln!(out, "{}", m.file.magenta())?;
            } else {
                writeln!(out, "{}", m.file)?;
            }
        }
    }
    Ok(())
}

fn print_count(matches: &[SearchMatch], out: &mut impl Write, color: bool) -> io::Result<()> {
    let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for m in matches {
        *counts.entry(&m.file).or_insert(0) += 1;
    }
    let mut sorted: Vec<_> = counts.into_iter().collect();
    sorted.sort_by_key(|(f, _)| f.to_string());
    for (file, count) in sorted {
        if color {
            writeln!(out, "{}:{}", file.magenta(), count.to_string().green())?;
        } else {
            writeln!(out, "{}:{}", file, count)?;
        }
    }
    Ok(())
}

fn print_grep_format(
    matches: &[SearchMatch],
    out: &mut impl Write,
    config: &OutputConfig,
) -> io::Result<()> {
    for m in matches {
        // Path-name match: line_number == 0 -> print bare path (rg --files style).
        if m.line_number == 0 {
            if config.color {
                writeln!(out, "{}", m.file.magenta())?;
            } else {
                writeln!(out, "{}", m.file)?;
            }
            continue;
        }
        if config.color {
            write!(out, "{}", m.file.magenta())?;
            if config.line_numbers {
                write!(out, "{}{}",":".cyan(), m.line_number.to_string().green())?;
            }
            writeln!(out, "{}{}", ":".cyan(), m.line_content)?;
        } else {
            if config.line_numbers {
                writeln!(out, "{}:{}:{}", m.file, m.line_number, m.line_content)?;
            } else {
                writeln!(out, "{}:{}", m.file, m.line_content)?;
            }
        }
    }
    Ok(())
}
