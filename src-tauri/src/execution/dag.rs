//! Directed Acyclic Graph for Operation Dependencies
//!
//! This module builds a DAG from WAL entries based on their dependencies,
//! computes execution levels for parallel execution, and provides
//! topological ordering with cycle detection.

use crate::wal::entry::WALEntry;
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;
use uuid::Uuid;

/// Error types for DAG operations
#[derive(Debug, Clone)]
pub enum DAGError {
    /// A cycle was detected in the dependency graph
    CycleDetected,
    /// A referenced dependency was not found
    DependencyNotFound(Uuid),
    /// The graph is empty
    EmptyGraph,
}

impl std::fmt::Display for DAGError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DAGError::CycleDetected => write!(f, "Cycle detected in operation dependencies"),
            DAGError::DependencyNotFound(id) => write!(f, "Dependency not found: {}", id),
            DAGError::EmptyGraph => write!(f, "Cannot build DAG from empty entry list"),
        }
    }
}

impl std::error::Error for DAGError {}

impl From<DAGError> for String {
    fn from(err: DAGError) -> Self {
        err.to_string()
    }
}

/// A Directed Acyclic Graph for managing operation execution order
///
/// Operations are nodes in the graph, and edges represent "must complete before"
/// relationships. The graph is organized into levels where each level's operations
/// can be executed in parallel.
#[derive(Debug)]
pub struct ExecutionDAG {
    /// The underlying petgraph structure
    graph: DiGraph<WALEntry, ()>,
    /// Mapping from entry UUID to graph node index
    id_to_index: HashMap<Uuid, NodeIndex>,
    /// Operations grouped by execution level (parallel groups)
    levels: Vec<Vec<NodeIndex>>,
}

impl ExecutionDAG {
    /// Create a new ExecutionDAG from a list of WAL entries
    ///
    /// The entries should include dependency information in their `depends_on` field.
    /// This function will:
    /// 1. Add all entries as nodes
    /// 2. Add edges based on dependencies
    /// 3. Verify no cycles exist
    /// 4. Compute execution levels
    pub fn from_entries(entries: Vec<WALEntry>) -> Result<Self, DAGError> {
        if entries.is_empty() {
            return Err(DAGError::EmptyGraph);
        }

        let mut graph: DiGraph<WALEntry, ()> = DiGraph::new();
        let mut id_to_index: HashMap<Uuid, NodeIndex> = HashMap::new();

        // First pass: add all entries as nodes
        for entry in &entries {
            let idx = graph.add_node(entry.clone());
            id_to_index.insert(entry.id, idx);
        }

        // Second pass: add edges based on dependencies
        for entry in &entries {
            let entry_idx = id_to_index[&entry.id];
            for dep_id in &entry.depends_on {
                let dep_idx = id_to_index.get(dep_id).ok_or(DAGError::DependencyNotFound(*dep_id))?;
                // Edge direction: dependency -> dependent
                // (the dependency must complete before the dependent can start)
                graph.add_edge(*dep_idx, entry_idx, ());
            }
        }

        // Verify no cycles using topological sort
        if toposort(&graph, None).is_err() {
            return Err(DAGError::CycleDetected);
        }

        let mut dag = ExecutionDAG {
            graph,
            id_to_index,
            levels: Vec::new(),
        };

        // Compute execution levels
        dag.compute_levels();

        Ok(dag)
    }

    /// Compute execution levels for parallel execution
    ///
    /// Level 0 contains nodes with no dependencies.
    /// Level N contains nodes whose dependencies are all in levels 0 to N-1.
    fn compute_levels(&mut self) {
        // Use the longest path algorithm to assign levels
        let mut node_levels: HashMap<NodeIndex, usize> = HashMap::new();

        // Get topologically sorted nodes
        let sorted = match toposort(&self.graph, None) {
            Ok(s) => s,
            Err(_) => return, // Should never happen as we checked for cycles
        };

        // Assign levels based on dependency depth
        for node_idx in sorted {
            let mut max_dep_level: Option<usize> = None;

            // Find the maximum level of all dependencies
            for neighbor in self.graph.neighbors_directed(node_idx, petgraph::Direction::Incoming) {
                let neighbor_level = node_levels.get(&neighbor).copied().unwrap_or(0);
                max_dep_level = Some(max_dep_level.map_or(neighbor_level, |m| m.max(neighbor_level)));
            }

            // This node's level is one more than its highest dependency
            let level = max_dep_level.map_or(0, |l| l + 1);
            node_levels.insert(node_idx, level);
        }

        // Group nodes by level
        let max_level = node_levels.values().max().copied().unwrap_or(0);
        let mut levels: Vec<Vec<NodeIndex>> = vec![Vec::new(); max_level + 1];

        for (node_idx, level) in node_levels {
            levels[level].push(node_idx);
        }

        self.levels = levels;
    }

    /// Get all execution levels
    ///
    /// Each level is a vector of entries that can be executed in parallel.
    /// Levels must be executed in order (level 0, then level 1, etc.).
    pub fn get_levels(&self) -> Vec<Vec<&WALEntry>> {
        self.levels
            .iter()
            .map(|level| {
                level
                    .iter()
                    .filter_map(|idx| self.graph.node_weight(*idx))
                    .collect()
            })
            .collect()
    }

    /// Get execution levels as owned entries (for async execution)
    pub fn get_levels_owned(&self) -> Vec<Vec<WALEntry>> {
        self.levels
            .iter()
            .map(|level| {
                level
                    .iter()
                    .filter_map(|idx| self.graph.node_weight(*idx).cloned())
                    .collect()
            })
            .collect()
    }

    /// Get the total number of entries in the DAG
    pub fn len(&self) -> usize {
        self.graph.node_count()
    }

    /// Check if the DAG is empty
    pub fn is_empty(&self) -> bool {
        self.graph.node_count() == 0
    }

    /// Get the number of execution levels
    pub fn level_count(&self) -> usize {
        self.levels.len()
    }

    /// Get a specific entry by ID
    pub fn get_entry(&self, id: Uuid) -> Option<&WALEntry> {
        self.id_to_index
            .get(&id)
            .and_then(|idx| self.graph.node_weight(*idx))
    }

    /// Get entries in topological order
    pub fn topological_order(&self) -> Vec<&WALEntry> {
        match toposort(&self.graph, None) {
            Ok(sorted) => sorted
                .iter()
                .filter_map(|idx| self.graph.node_weight(*idx))
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Get entries that depend on a specific entry
    pub fn get_dependents(&self, id: Uuid) -> Vec<&WALEntry> {
        match self.id_to_index.get(&id) {
            Some(idx) => self
                .graph
                .neighbors_directed(*idx, petgraph::Direction::Outgoing)
                .filter_map(|dep_idx| self.graph.node_weight(dep_idx))
                .collect(),
            None => Vec::new(),
        }
    }

    /// Get entries that an entry depends on
    pub fn get_dependencies(&self, id: Uuid) -> Vec<&WALEntry> {
        match self.id_to_index.get(&id) {
            Some(idx) => self
                .graph
                .neighbors_directed(*idx, petgraph::Direction::Incoming)
                .filter_map(|dep_idx| self.graph.node_weight(dep_idx))
                .collect(),
            None => Vec::new(),
        }
    }

    /// Compute statistics about the DAG
    pub fn stats(&self) -> DAGStats {
        let level_sizes: Vec<usize> = self.levels.iter().map(|l| l.len()).collect();
        let max_parallelism = level_sizes.iter().max().copied().unwrap_or(0);

        DAGStats {
            total_entries: self.len(),
            level_count: self.level_count(),
            level_sizes,
            max_parallelism,
        }
    }
}

/// Statistics about the DAG structure
#[derive(Debug, Clone)]
pub struct DAGStats {
    /// Total number of entries
    pub total_entries: usize,
    /// Number of execution levels
    pub level_count: usize,
    /// Size of each level
    pub level_sizes: Vec<usize>,
    /// Maximum number of operations that can run in parallel
    pub max_parallelism: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wal::entry::{WALEntry, WALOperationType};
    use std::path::PathBuf;

    fn create_test_entry(sequence: u32, deps: Vec<Uuid>) -> WALEntry {
        WALEntry::new_with_deps(
            WALOperationType::CreateFolder {
                path: PathBuf::from(format!("/test/folder{}", sequence)),
            },
            sequence,
            deps,
        )
        .expect("Failed to create test entry")
    }

    #[test]
    fn test_simple_dag() {
        let entry1 = create_test_entry(0, vec![]);
        let entry2 = create_test_entry(1, vec![]);

        let dag = ExecutionDAG::from_entries(vec![entry1, entry2]).unwrap();

        assert_eq!(dag.len(), 2);
        // Both entries have no dependencies, so they should be in level 0
        assert_eq!(dag.level_count(), 1);
        assert_eq!(dag.get_levels()[0].len(), 2);
    }

    #[test]
    fn test_sequential_dag() {
        let entry1 = create_test_entry(0, vec![]);
        let entry2 = create_test_entry(1, vec![entry1.id]);
        let entry3 = create_test_entry(2, vec![entry2.id]);

        let dag = ExecutionDAG::from_entries(vec![
            entry1.clone(),
            entry2.clone(),
            entry3.clone(),
        ])
        .unwrap();

        assert_eq!(dag.len(), 3);
        assert_eq!(dag.level_count(), 3);
        assert_eq!(dag.get_levels()[0].len(), 1);
        assert_eq!(dag.get_levels()[1].len(), 1);
        assert_eq!(dag.get_levels()[2].len(), 1);
    }

    #[test]
    fn test_diamond_dag() {
        // Diamond dependency:
        //     A
        //    / \
        //   B   C
        //    \ /
        //     D
        let a = create_test_entry(0, vec![]);
        let b = create_test_entry(1, vec![a.id]);
        let c = create_test_entry(2, vec![a.id]);
        let d = create_test_entry(3, vec![b.id, c.id]);

        let dag = ExecutionDAG::from_entries(vec![
            a.clone(),
            b.clone(),
            c.clone(),
            d.clone(),
        ])
        .unwrap();

        assert_eq!(dag.len(), 4);
        assert_eq!(dag.level_count(), 3);

        let levels = dag.get_levels();
        assert_eq!(levels[0].len(), 1); // A
        assert_eq!(levels[1].len(), 2); // B, C (parallel)
        assert_eq!(levels[2].len(), 1); // D
    }

    #[test]
    fn test_cycle_detection() {
        // Create a cycle: A -> B -> C -> A
        let mut a = create_test_entry(0, vec![]);
        let b = create_test_entry(1, vec![a.id]);
        let c = create_test_entry(2, vec![b.id]);
        a.depends_on = vec![c.id]; // Create cycle

        let result = ExecutionDAG::from_entries(vec![a, b, c]);
        assert!(matches!(result, Err(DAGError::CycleDetected)));
    }

    #[test]
    fn test_empty_entries() {
        let result = ExecutionDAG::from_entries(vec![]);
        assert!(matches!(result, Err(DAGError::EmptyGraph)));
    }

    #[test]
    fn test_missing_dependency() {
        let fake_id = Uuid::new_v4();
        let entry = create_test_entry(0, vec![fake_id]);

        let result = ExecutionDAG::from_entries(vec![entry]);
        assert!(matches!(result, Err(DAGError::DependencyNotFound(_))));
    }

    #[test]
    fn test_topological_order() {
        let a = create_test_entry(0, vec![]);
        let b = create_test_entry(1, vec![a.id]);
        let c = create_test_entry(2, vec![b.id]);

        let dag = ExecutionDAG::from_entries(vec![
            a.clone(),
            b.clone(),
            c.clone(),
        ])
        .unwrap();

        let order = dag.topological_order();
        assert_eq!(order.len(), 3);
        // A must come before B, B must come before C
        let a_pos = order.iter().position(|e| e.id == a.id).unwrap();
        let b_pos = order.iter().position(|e| e.id == b.id).unwrap();
        let c_pos = order.iter().position(|e| e.id == c.id).unwrap();
        assert!(a_pos < b_pos);
        assert!(b_pos < c_pos);
    }

    #[test]
    fn test_stats() {
        let a = create_test_entry(0, vec![]);
        let b = create_test_entry(1, vec![a.id]);
        let c = create_test_entry(2, vec![a.id]);

        let dag = ExecutionDAG::from_entries(vec![a, b, c]).unwrap();

        let stats = dag.stats();
        assert_eq!(stats.total_entries, 3);
        assert_eq!(stats.level_count, 2);
        assert_eq!(stats.max_parallelism, 2); // B and C can run in parallel
    }
}
