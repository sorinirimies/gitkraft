//! Graph layout computation — lane-based algorithm for commit graph visualisation.
//!
//! The algorithm assigns colours **per lane** (not per OID) so that a linear
//! branch keeps a single consistent colour.  New colours are only introduced
//! when a new lane is created for a branch or merge.

use std::collections::HashSet;

use super::types::{GraphEdge, GraphRow};
use crate::features::commits::CommitInfo;

/// Build a graph layout for a slice of commits (newest-first).
///
/// The algorithm maintains a set of "active lanes" — columns that carry a
/// branch line downward through the graph.  Each lane tracks the OID of the
/// next commit it expects to encounter and its colour.
///
/// Colours are assigned per-lane:
/// - When a lane is created (new branch / new root), it gets a fresh colour.
/// - When a commit takes over a lane, it inherits that lane's colour.
/// - First-parent edges continue the lane colour.
/// - Additional-parent edges get the colour of their target lane.
pub fn build_graph(commits: &[CommitInfo]) -> Vec<GraphRow> {
    if commits.is_empty() {
        return Vec::new();
    }

    /// An active lane: the OID it's waiting for, and its colour index.
    struct Lane {
        target_oid: String,
        color: usize,
    }

    let mut lanes: Vec<Option<Lane>> = Vec::new();
    let mut next_color: usize = 0;
    let mut rows: Vec<GraphRow> = Vec::with_capacity(commits.len());

    // Pre-build a set of OIDs in this commit list so we can detect when a
    // parent is NOT in the loaded window (and thus its lane will never resolve).
    let oid_set: HashSet<&str> = commits
        .iter()
        .map(|c| c.oid.as_str())
        .collect();

    for commit in commits {
        let oid = &commit.oid;

        // Find the lane expecting this commit, or allocate a new one.
        let node_col = lanes
            .iter()
            .position(|l| l.as_ref().is_some_and(|ln| ln.target_oid == *oid))
            .unwrap_or_else(|| {
                let color = next_color;
                next_color += 1;
                let idx = lanes.iter().position(|l| l.is_none()).unwrap_or_else(|| {
                    lanes.push(None);
                    lanes.len() - 1
                });
                lanes[idx] = Some(Lane {
                    target_oid: oid.clone(),
                    color,
                });
                idx
            });

        // The node inherits its lane's colour.
        let node_color = lanes[node_col].as_ref().map(|l| l.color).unwrap_or(0);

        // Build edges.
        let mut edges: Vec<GraphEdge> = Vec::new();

        // Pass-through edges for lanes that are not the node lane.
        for (col, lane_opt) in lanes.iter().enumerate() {
            if col == node_col {
                continue;
            }
            if let Some(lane) = lane_opt {
                edges.push(GraphEdge {
                    from_column: col,
                    to_column: col,
                    color_index: lane.color,
                });
            }
        }

        // Handle the commit's parents.
        let parents = &commit.parent_ids;

        if parents.is_empty() {
            // Root commit — the lane ends.
            lanes[node_col] = None;
        } else {
            // First parent takes over the node's lane (same colour).
            let first_parent = &parents[0];
            let lane_color = node_color;

            lanes[node_col] = Some(Lane {
                target_oid: first_parent.clone(),
                color: lane_color,
            });

            edges.push(GraphEdge {
                from_column: node_col,
                to_column: node_col,
                color_index: lane_color,
            });

            // Additional parents get new lanes (or reuse existing ones).
            for parent_oid in &parents[1..] {
                let existing = lanes
                    .iter()
                    .position(|l| l.as_ref().is_some_and(|ln| ln.target_oid == *parent_oid));

                let (target_col, target_color) = if let Some(col) = existing {
                    let color = lanes[col].as_ref().unwrap().color;
                    (col, color)
                } else {
                    let color = next_color;
                    next_color += 1;
                    let idx = lanes.iter().position(|l| l.is_none()).unwrap_or_else(|| {
                        lanes.push(None);
                        lanes.len() - 1
                    });
                    lanes[idx] = Some(Lane {
                        target_oid: parent_oid.clone(),
                        color,
                    });
                    (idx, color)
                };

                edges.push(GraphEdge {
                    from_column: node_col,
                    to_column: target_col,
                    color_index: target_color,
                });
            }
        }

        // Clean up lanes whose target is NOT in our commit list — they'll
        // never resolve, so keeping them wastes columns.  But only clean
        // lanes that are NOT the node's lane (we just set that above).
        for (col, lane_opt) in lanes.iter_mut().enumerate() {
            if col == node_col {
                continue;
            }
            if let Some(lane) = lane_opt {
                if !oid_set.contains(lane.target_oid.as_str()) {
                    *lane_opt = None;
                }
            }
        }

        // Compact: remove trailing empty lanes.
        while lanes.last().is_some_and(|l| l.is_none()) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_commit(oid: &str, parents: &[&str]) -> CommitInfo {
        CommitInfo {
            oid: oid.to_string(),
            summary: format!("commit {oid}"),
            message: format!("commit {oid}"),
            author_name: "Test".to_string(),
            author_email: "test@test.com".to_string(),
            time: Utc::now(),
            parent_ids: parents.iter().map(|s| s.to_string()).collect(),
            refs: Vec::new(),
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
    fn linear_history_same_colour() {
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
        // All nodes should share the same colour (lane-based colouring).
        assert_eq!(rows[0].node_color, rows[1].node_color);
        assert_eq!(rows[1].node_color, rows[2].node_color);
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
    fn merge_creates_different_colour_for_second_parent() {
        let commits = vec![
            make_commit("merge00", &["parent1", "parent2"]),
            make_commit("parent2", &["base000"]),
            make_commit("parent1", &["base000"]),
            make_commit("base000", &[]),
        ];
        let rows = build_graph(&commits);
        let merge_edges = &rows[0].edges;
        let cross_edges: Vec<_> = merge_edges
            .iter()
            .filter(|e| e.from_column != e.to_column)
            .collect();
        assert!(!cross_edges.is_empty(), "merge must have a cross-edge");
        assert_ne!(cross_edges[0].color_index, rows[0].node_color);
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
            assert!(row.width > row.node_column);
        }
    }
}
