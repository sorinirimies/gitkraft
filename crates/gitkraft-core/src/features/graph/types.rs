use serde::{Deserialize, Serialize};

/// A commit node with its position in the graph visual.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphCommit {
    /// The commit data.
    pub commit: crate::features::commits::CommitInfo,
    /// Which column (lane) this commit sits in (0-based).
    pub column: usize,
    /// Edges from this node to its parents.
    pub edges: Vec<GraphEdge>,
}

/// An edge connecting a child commit to a parent commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Column of the child (source) node.
    pub from_column: usize,
    /// Column of the parent (target) node.
    pub to_column: usize,
    /// The color index (used for consistent coloring per branch lane).
    pub color_index: usize,
}

/// A row in the graph — represents what to draw for one commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphRow {
    /// How many columns wide the graph is at this row.
    pub width: usize,
    /// The column the commit node sits in.
    pub node_column: usize,
    /// The node color index.
    pub node_color: usize,
    /// Edges passing through or starting/ending at this row.
    pub edges: Vec<GraphEdge>,
}
