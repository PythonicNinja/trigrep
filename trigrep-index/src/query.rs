use std::collections::HashSet;
use crate::error::IndexError;
use crate::reader::IndexReader;
use crate::types::QueryPlan;

/// Execute a query plan against the index, returning candidate file IDs.
pub fn execute(reader: &mut IndexReader, plan: &QueryPlan) -> Result<Vec<u32>, IndexError> {
    match plan {
        QueryPlan::MatchAll => Ok((0..reader.num_files() as u32).collect()),

        QueryPlan::And(trigrams) => {
            if trigrams.is_empty() {
                return Ok((0..reader.num_files() as u32).collect());
            }

            // Load all posting lists, sort by length (smallest first)
            let mut lists: Vec<Vec<u32>> = Vec::new();
            for tq in trigrams {
                let postings = reader.read_posting_list(tq.hash)?;
                let file_ids: Vec<u32> = postings.iter().map(|p| p.file_id).collect();
                lists.push(file_ids);
            }
            lists.sort_by_key(|l| l.len());

            // Intersect progressively from smallest
            let mut candidates: HashSet<u32> = lists[0].iter().copied().collect();
            for list in &lists[1..] {
                let other: HashSet<u32> = list.iter().copied().collect();
                candidates.retain(|id| other.contains(id));
                if candidates.is_empty() {
                    break;
                }
            }

            let mut result: Vec<u32> = candidates.into_iter().collect();
            result.sort();
            Ok(result)
        }

        QueryPlan::Or(branches) => {
            let mut all: HashSet<u32> = HashSet::new();
            for branch in branches {
                let ids = execute(reader, branch)?;
                all.extend(ids);
            }
            let mut result: Vec<u32> = all.into_iter().collect();
            result.sort();
            Ok(result)
        }
    }
}
