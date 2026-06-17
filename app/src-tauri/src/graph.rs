use petgraph::graph::{DiGraph, NodeIndex};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub struct KnowledgeGraph {
    graph: DiGraph<String, ()>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub links: Vec<GraphLink>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GraphNode {
    pub id: String,
    pub title: String,
    pub dangling: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash)]
pub struct GraphLink {
    pub source: String,
    pub target: String,
}

pub fn build(notes: &[(String, Vec<String>, String)]) -> GraphData {
    KnowledgeGraph::build(notes)
}

impl KnowledgeGraph {
    pub fn build(notes: &[(String, Vec<String>, String)]) -> GraphData {
        let mut graph = DiGraph::new();
        let mut indices: HashMap<String, NodeIndex> = HashMap::new();
        let mut nodes = Vec::new();
        let mut links = Vec::new();
        let mut link_set = HashSet::new();
        let mut title_index = HashMap::new();
        let mut stem_index = HashMap::new();

        for (path, _, title) in notes {
            let title = display_title(path, title);
            title_index.insert(normalize_key(&title), path.clone());
            if let Some(stem) = Path::new(path).file_stem().and_then(|stem| stem.to_str()) {
                stem_index.insert(normalize_key(stem), path.clone());
            }

            let index = graph.add_node(path.clone());
            indices.insert(path.clone(), index);
            nodes.push(GraphNode {
                id: path.clone(),
                title,
                dangling: false,
            });
        }

        for (source, wikilinks, _) in notes {
            for target in wikilinks {
                let target_id = resolve_target(target, &title_index, &stem_index)
                    .unwrap_or_else(|| target.clone());

                let target_index = if let Some(index) = indices.get(&target_id) {
                    *index
                } else {
                    let index = graph.add_node(target_id.clone());
                    indices.insert(target_id.clone(), index);
                    nodes.push(GraphNode {
                        id: target_id.clone(),
                        title: target.clone(),
                        dangling: true,
                    });
                    index
                };

                let Some(source_index) = indices.get(source).copied() else {
                    continue;
                };

                if link_set.insert((source.clone(), target_id.clone())) {
                    graph.add_edge(source_index, target_index, ());
                    links.push(GraphLink {
                        source: source.clone(),
                        target: target_id,
                    });
                }
            }
        }

        let _knowledge_graph = KnowledgeGraph { graph };
        GraphData { nodes, links }
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }
}

fn resolve_target(
    target: &str,
    title_index: &HashMap<String, String>,
    stem_index: &HashMap<String, String>,
) -> Option<String> {
    let key = normalize_key(target);
    title_index
        .get(&key)
        .or_else(|| stem_index.get(&key))
        .cloned()
}

fn display_title(path: &str, title: &str) -> String {
    if !title.trim().is_empty() {
        return title.to_string();
    }
    Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(path)
        .to_string()
}

fn normalize_key(value: &str) -> String {
    value
        .trim()
        .trim_end_matches(".md")
        .trim_end_matches(".markdown")
        .to_lowercase()
}
