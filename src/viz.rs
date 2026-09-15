use std::collections::HashMap;

use crate::graph::GodNode;

const TEMPLATE: &str = include_str!("templates/graph.html");

const CLUSTER_PALETTE: &[&str] = &[
    "#7aa2f7", "#bb9af7", "#7dcfff", "#9ece6a", "#e0af68", "#f7768e", "#73daca", "#ff9e64",
    "#2ac3de", "#b4f9f8", "#c0caf5", "#a9b1d6",
];

pub fn generate_html(
    vault_name: &str,
    outgoing: &HashMap<String, Vec<String>>,
    community_of: &HashMap<String, usize>,
    communities: &[Vec<String>],
    god_nodes: &[GodNode],
) -> String {
    let mut importance: HashMap<&str, f64> = HashMap::new();
    for gn in god_nodes {
        importance.insert(&gn.name, gn.score);
    }

    let mut degree: HashMap<String, usize> = HashMap::new();
    let mut all_nodes = std::collections::HashSet::new();
    for (node, targets) in outgoing {
        all_nodes.insert(node.clone());
        *degree.entry(node.clone()).or_default() += targets.len();
        for t in targets {
            all_nodes.insert(t.clone());
            *degree.entry(t.clone()).or_default() += 1;
        }
    }

    let nodes_json: Vec<String> = all_nodes
        .iter()
        .map(|name| {
            let group = community_of.get(name).copied().unwrap_or(0);
            let size = importance.get(name.as_str()).copied().unwrap_or(0.0);
            let deg = degree.get(name).copied().unwrap_or(0);
            format!(
                r#"{{"id":"{}","label":"{}","group":{},"size":{:.3},"degree":{}}}"#,
                escape_json(name),
                escape_json(name),
                group,
                size,
                deg
            )
        })
        .collect();

    let mut edges_json = Vec::new();
    for (source, targets) in outgoing {
        for target in targets {
            edges_json.push(format!(
                r#"{{"from":"{}","to":"{}"}}"#,
                escape_json(source),
                escape_json(target)
            ));
        }
    }

    let colors_json: Vec<String> = (0..communities.len().max(1))
        .map(|i| format!(r#""{}""#, CLUSTER_PALETTE[i % CLUSTER_PALETTE.len()]))
        .collect();
    let labels_json: Vec<String> = communities
        .iter()
        .map(|members| {
            let best = members
                .iter()
                .max_by_key(|m| degree.get(*m).copied().unwrap_or(0))
                .cloned()
                .unwrap_or_default();
            format!(r#""{}""#, escape_json(&best))
        })
        .collect();

    TEMPLATE
        .replace("{{VAULT_NAME}}", &escape_html(vault_name))
        .replace("{{GRAPH_NODES}}", &format!("[{}]", nodes_json.join(",")))
        .replace("{{GRAPH_EDGES}}", &format!("[{}]", edges_json.join(",")))
        .replace(
            "{{CLUSTER_COLORS}}",
            &format!("[{}]", colors_json.join(",")),
        )
        .replace(
            "{{CLUSTER_LABELS}}",
            &format!("[{}]", labels_json.join(",")),
        )
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
