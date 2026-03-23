use regex_syntax::hir::{Hir, HirKind};
use trigrep_index::types::{trigram_hash, QueryPlan, TrigramQuery};

/// Decompose a regex pattern into a QueryPlan of trigram lookups.
pub fn decompose(pattern: &str, case_insensitive: bool) -> QueryPlan {
    // Case-insensitive queries can't use the raw-byte index effectively
    // since the index stores original case. Fall back to scanning all files.
    if case_insensitive {
        return QueryPlan::MatchAll;
    }

    let hir = match regex_syntax::parse(pattern) {
        Ok(h) => h,
        Err(_) => return QueryPlan::MatchAll,
    };
    let plan = extract_query(&hir, case_insensitive);
    simplify(plan)
}

fn extract_query(hir: &Hir, case_insensitive: bool) -> QueryPlan {
    match hir.kind() {
        HirKind::Literal(lit) => {
            let bytes = if case_insensitive {
                lit.0.to_ascii_lowercase()
            } else {
                lit.0.to_vec()
            };
            literals_to_trigrams(&bytes)
        }

        HirKind::Concat(subs) => {
            // Extract literal runs from the concatenation
            let mut all_bytes = Vec::new();
            let mut plans = Vec::new();

            for sub in subs {
                if let HirKind::Literal(lit) = sub.kind() {
                    let bytes = if case_insensitive {
                        lit.0.to_ascii_lowercase()
                    } else {
                        lit.0.to_vec()
                    };
                    all_bytes.extend_from_slice(&bytes);
                } else {
                    // Flush accumulated literal bytes
                    if !all_bytes.is_empty() {
                        let plan = literals_to_trigrams(&all_bytes);
                        if !matches!(plan, QueryPlan::MatchAll) {
                            plans.push(plan);
                        }
                        all_bytes.clear();
                    }
                    // Recurse into non-literal
                    let sub_plan = extract_query(sub, case_insensitive);
                    if !matches!(sub_plan, QueryPlan::MatchAll) {
                        plans.push(sub_plan);
                    }
                }
            }
            // Flush remaining
            if !all_bytes.is_empty() {
                let plan = literals_to_trigrams(&all_bytes);
                if !matches!(plan, QueryPlan::MatchAll) {
                    plans.push(plan);
                }
            }

            match plans.len() {
                0 => QueryPlan::MatchAll,
                1 => plans.into_iter().next().unwrap(),
                _ => {
                    // Flatten nested ANDs
                    let mut flat = Vec::new();
                    for p in plans {
                        if let QueryPlan::And(inner) = p {
                            flat.extend(inner);
                        } else if let QueryPlan::Or(_) = p {
                            // Keep OR branches as-is — we'd need a more complex structure
                            // For Phase 1, just wrap it
                            flat.push(TrigramQuery {
                                hash: 0,
                                expected_next: None,
                            });
                            // Actually, just return a combined AND of all plans
                            return QueryPlan::And(flat);
                        }
                    }
                    QueryPlan::And(flat)
                }
            }
        }

        HirKind::Alternation(branches) => {
            let plans: Vec<QueryPlan> = branches
                .iter()
                .map(|b| extract_query(b, case_insensitive))
                .collect();

            // If any branch is MatchAll, the whole alternation is MatchAll
            if plans.iter().any(|p| matches!(p, QueryPlan::MatchAll)) {
                return QueryPlan::MatchAll;
            }

            QueryPlan::Or(plans)
        }

        HirKind::Repetition(rep) => {
            if rep.min >= 1 {
                extract_query(&rep.sub, case_insensitive)
            } else {
                QueryPlan::MatchAll
            }
        }

        HirKind::Capture(cap) => extract_query(&cap.sub, case_insensitive),

        // Class, Look, Empty — no extractable literals
        _ => QueryPlan::MatchAll,
    }
}

/// Convert a byte sequence into a QueryPlan of AND'd trigram queries.
fn literals_to_trigrams(bytes: &[u8]) -> QueryPlan {
    if bytes.len() < 3 {
        return QueryPlan::MatchAll;
    }

    let trigrams: Vec<TrigramQuery> = (0..bytes.len() - 2)
        .map(|i| {
            let hash = trigram_hash(bytes[i], bytes[i + 1], bytes[i + 2]);
            let expected_next = if i + 3 < bytes.len() {
                Some(bytes[i + 3])
            } else {
                None
            };
            TrigramQuery {
                hash,
                expected_next,
            }
        })
        .collect();

    QueryPlan::And(trigrams)
}

/// Simplify a query plan by removing redundancies.
fn simplify(plan: QueryPlan) -> QueryPlan {
    match plan {
        QueryPlan::And(trigrams) if trigrams.is_empty() => QueryPlan::MatchAll,
        QueryPlan::Or(branches) if branches.len() == 1 => {
            simplify(branches.into_iter().next().unwrap())
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_literal() {
        let plan = decompose("hello", false);
        match plan {
            QueryPlan::And(trigrams) => {
                // "hello" -> "hel", "ell", "llo" = 3 trigrams
                assert_eq!(trigrams.len(), 3);
            }
            _ => panic!("Expected And plan for plain literal"),
        }
    }

    #[test]
    fn test_short_literal() {
        let plan = decompose("ab", false);
        assert!(matches!(plan, QueryPlan::MatchAll));
    }

    #[test]
    fn test_alternation() {
        let plan = decompose("foo|bar", false);
        match plan {
            QueryPlan::Or(branches) => {
                assert_eq!(branches.len(), 2);
            }
            _ => panic!("Expected Or plan for alternation"),
        }
    }

    #[test]
    fn test_dot_star_between_literals() {
        let plan = decompose("foo.*bar", false);
        // Should extract trigrams from "foo" and "bar" in an AND
        match plan {
            QueryPlan::And(trigrams) => {
                assert!(trigrams.len() >= 2); // at least 1 from "foo" + 1 from "bar"
            }
            _ => panic!("Expected And plan, got {:?}", plan),
        }
    }

    #[test]
    fn test_case_insensitive_falls_back() {
        // Case-insensitive queries fall back to MatchAll since the index
        // stores raw bytes and can't efficiently match case-folded trigrams.
        let plan = decompose("Hello", true);
        assert!(matches!(plan, QueryPlan::MatchAll));
    }
}
