use crate::db;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub struct KnowledgeGraph {
    graph: DiGraph<String, ()>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub links: Vec<GraphLink>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GraphNode {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub dangling: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash)]
pub struct GraphLink {
    pub source: String,
    pub target: String,
    pub kind: String,
}

pub fn build(notes: &[(String, Vec<String>, String)]) -> GraphData {
    KnowledgeGraph::build(notes)
}

pub fn build_from_db(conn: &db::Connection) -> db::Result<GraphData> {
    let files = db::list_files(conn)?;
    let links = db::list_links(conn)?;
    let mut graph = DiGraph::new();
    let mut indices: HashMap<String, NodeIndex> = HashMap::new();
    let mut nodes = Vec::new();
    let mut graph_links = Vec::new();
    let mut known_paths = HashSet::new();
    let mut link_set = HashSet::new();

    for file in files {
        known_paths.insert(file.path.clone());
        let index = graph.add_node(file.path.clone());
        indices.insert(file.path.clone(), index);
        nodes.push(GraphNode {
            id: file.path.clone(),
            title: display_title(&file.path, &file.title),
            kind: file.kind,
            dangling: false,
        });
    }

    for link in links {
        let source_index = match indices.get(&link.source_path).copied() {
            Some(index) => index,
            None => {
                let index = graph.add_node(link.source_path.clone());
                indices.insert(link.source_path.clone(), index);
                nodes.push(GraphNode {
                    id: link.source_path.clone(),
                    title: basename_title(&link.source_path),
                    kind: "dangling".to_string(),
                    dangling: true,
                });
                index
            }
        };

        let target_index = match indices.get(&link.target_path).copied() {
            Some(index) => index,
            None => {
                let index = graph.add_node(link.target_path.clone());
                indices.insert(link.target_path.clone(), index);
                let kind = if link.resolved && known_paths.contains(&link.target_path) {
                    "text"
                } else if link.target_path.starts_with("external:") {
                    "external"
                } else {
                    "dangling"
                };
                nodes.push(GraphNode {
                    id: link.target_path.clone(),
                    title: basename_title(&link.target_path),
                    kind: kind.to_string(),
                    dangling: !link.resolved,
                });
                index
            }
        };

        if link_set.insert((
            link.source_path.clone(),
            link.target_path.clone(),
            link.kind.clone(),
        )) {
            graph.add_edge(source_index, target_index, ());
            graph_links.push(GraphLink {
                source: link.source_path,
                target: link.target_path,
                kind: link.kind,
            });
        }
    }

    let _knowledge_graph = KnowledgeGraph { graph };
    Ok(GraphData {
        nodes,
        links: graph_links,
    })
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
                kind: "text".to_string(),
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
                        kind: "dangling".to_string(),
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
                        kind: "Wikilink".to_string(),
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

fn basename_title(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
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
