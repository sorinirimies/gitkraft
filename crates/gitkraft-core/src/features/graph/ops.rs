//! Graph layout computation — lane-based algorithm for commit graph visualisation.

use anyhow::Result;
use git2::Repository;
use std::collections::HashMap;

use super::types::{GraphEdge, GraphRow};
use crate::features::commits::CommitInfo;

/// Build a graph layout for a slice of commits (newest-first).
///
/// The algorithm maintains a set of "active lanes" — columns that carry a
/// branch line downward through the graph.  Each lane tracks the OID of the
/// next commit it expects to encounter.  When a commit is found, its lane
/// becomes the *node column*, and the commit's parents are assigned to lanes
/// (using the current lane for the first parent and allocating new lanes for
/// any additional parents).
pub fn build_graph(commits: &[CommitInfo]) -> Vec<GraphRow> {
    if commits.is_empty() {
        return Vec::new();
    }

    let mut lanes: Vec<Option<String>> = Vec::new();
    let mut color_map: HashMap<String, usize> = HashMap::new();
    let mut next_color: usize = 0;
    let mut rows: Vec<GraphRow> = Vec::with_capacity(commits.len());

    for commit in commits {
        let oid = &commit.oid;

        // Find or allocate the lane for this commit
        let node_col = lanes
            .iter()
            .position(|l| l.as_deref() == Some(oid))
            .unwrap_or_else(|| {
                lanes.push(Some(oid.clone()));
                lanes.len() - 1
            });

        // Assign a colour
        let node_color = *color_map.entry(oid.clone()).or_insert_with(|| {
            let c = next_color;
            next_color += 1;
            c
        });

        // Build edges
        let mut edges: Vec<GraphEdge> = Vec::new();

        // Pass-through edges for lanes that are not the node lane
        for (col, lane) in lanes.iter().enumerate() {
            if col == node_col {
                continue;
            }
            if let Some(ref target_oid) = lane {
                let lane_color = *color_map.entry(target_oid.clone()).or_insert_with(|| {
                    let c = next_color;
                    next_color += 1;
                    c
                });
                edges.push(GraphEdge {
                    from_column: col,
                    to_column: col,
                    color_index: lane_color,
                });
            }
        }

        // Handle the commit's parents
        let parents = &commit.parent_ids;

        if parents.is_empty() {
            // Root commit — the lane simply ends.
            lanes[node_col] = None;
        } else {
            // First parent takes over the node's lane.
            let first_parent = &parents[0];
            lanes[node_col] = Some(first_parent.clone());

            let first_parent_color = *color_map.entry(first_parent.clone()).or_insert_with(|| {
                let c = next_color;
                next_color += 1;
                c
            });

            edges.push(GraphEdge {
                from_column: node_col,
                to_column: node_col,
                color_index: first_parent_color,
            });

            // Additional parents get new lanes (or reuse existing ones).
            for parent_oid in &parents[1..] {
                let existing = lanes
                    .iter()
                    .position(|l| l.as_deref() == Some(parent_oid.as_str()));

                let target_col = if let Some(col) = existing {
                    col
                } else if let Some(free) = lanes.iter().position(|l| l.is_none()) {
                    lanes[free] = Some(parent_oid.clone());
                    free
                } else {
                    lanes.push(Some(parent_oid.clone()));
                    lanes.len() - 1
                };

                let parent_color = *color_map.entry(parent_oid.clone()).or_insert_with(|| {
                    let c = next_color;
                    next_color += 1;
                    c
                });

                edges.push(GraphEdge {
                    from_column: node_col,
                    to_column: target_col,
                    color_index: parent_color,
                });
            }
        }

        // Compact: remove trailing None lanes
        while lanes.last() == Some(&None) {
            lanes.pop();
        }

        let width = lanes.len().max(1);

        rows.push(GraphRow {
            width,
            node_column: node_col,
            node_color,
            edges,
        });
    }

    rows
}

/// Convenience: build graph directly from a repository.
pub fn build_graph_from_repo(
    repo: &Repository,
    max_count: usize,
) -> Result<(Vec<CommitInfo>, Vec<GraphRow>)> {
    let commits = crate::features::commits::list_commits(repo, max_count)?;
    let graph = build_graph(&commits);
    Ok((commits, graph))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_commit(oid: &str, parents: &[&str]) -> CommitInfo {
        CommitInfo {
            oid: oid.to_string(),
            short_oid: oid[..7.min(oid.len())].to_string(),
            summary: format!("commit {oid}"),
            message: format!("commit {oid}"),
            author_name: "Test".to_string(),
            author_email: "test@test.com".to_string(),
            time: Utc::now(),
            parent_ids: parents.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn empty_commits_returns_empty_graph() {
        let rows = build_graph(&[]);
        assert!(rows.is_empty());
    }

    #[test]
    fn single_commit_no_parents() {
        let commits = vec![make_commit("aaa0000", &[])];
        let rows = build_graph(&commits);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].node_column, 0);
        assert!(rows[0].edges.is_empty());
    }

    #[test]
    fn linear_history() {
        let commits = vec![
            make_commit("ccc0000", &["bbb0000"]),
            make_commit("bbb0000", &["aaa0000"]),
            make_commit("aaa0000", &[]),
        ];
        let rows = build_graph(&commits);
        assert_eq!(rows.len(), 3);
        for row in &rows {
            assert_eq!(row.node_column, 0);
        }
    }

    #[test]
    fn branch_uses_separate_lane() {
        let commits = vec![
            make_commit("ddd0000", &["bbb0000", "ccc0000"]),
            make_commit("ccc0000", &["aaa0000"]),
            make_commit("bbb0000", &["aaa0000"]),
            make_commit("aaa0000", &[]),
        ];
        let rows = build_graph(&commits);
        assert_eq!(rows.len(), 4);
        let merge_row = &rows[0];
        assert!(merge_row.edges.len() >= 2);
    }

    #[test]
    fn graph_width_is_at_least_node_column_plus_one() {
        let commits = vec![
            make_commit("ccc0000", &["bbb0000"]),
            make_commit("bbb0000", &["aaa0000"]),
            make_commit("aaa0000", &[]),
        ];
        let rows = build_graph(&commits);
        for row in &rows {
            assert!(row.width >= row.node_column + 1);
        }
    }
}
