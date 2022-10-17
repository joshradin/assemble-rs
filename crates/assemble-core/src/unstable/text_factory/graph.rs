//! Display graphs
//!
//! Provides overlays over ptree

use petgraph::graph::{IndexType, NodeIndex};
use petgraph::{EdgeType, Graph};
use ptree::{Color, IndentChars, PrintConfig};
use std::fmt::{Display, Formatter};

/// Creates the print config used throughout assemble
pub fn config() -> PrintConfig {
    let mut config = PrintConfig::default();
    config.indent = 4;
    config.characters = IndentChars {
        down_and_right: "+".to_string(),
        down: "|".to_string(),
        turn_right: "\\".to_string(),
        right: "-".to_string(),
        empty: " ".to_string()
    };
    config
}

/// Wrapper around a graph for accessing display
#[derive(Debug)]
pub struct PrettyGraph<'g, N: Clone, E : Clone, Ty: EdgeType, Ix: IndexType>(&'g Graph<N, E, Ty, Ix>, NodeIndex<Ix>);

impl<'g, N: Clone, E: Clone, Ty: EdgeType, Ix: IndexType> PrettyGraph<'g, N, E, Ty, Ix> {
    pub fn new(graph: &'g Graph<N, E, Ty, Ix>, root: NodeIndex<Ix>) -> Self {
        Self(graph, root)
    }
}

impl<'g, N: Clone + Display, E: Clone, Ty: EdgeType, Ix: IndexType> Display
    for PrettyGraph<'g, N, E, Ty, Ix>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut buffer = Vec::new();
        let mut config = config();
        ptree::graph::write_graph_with(&self.0, self.1, &mut buffer, &config)
            .map_err(|_| std::fmt::Error)?;
        let string = String::from_utf8(buffer).map_err(|_| std::fmt::Error)?;
        write!(f, "{}", string)
    }
}
