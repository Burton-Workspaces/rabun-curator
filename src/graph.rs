use std::collections::{HashMap, HashSet, VecDeque};

pub fn to_undirected(
    outgoing: &HashMap<String, Vec<String>>,
    incoming: &HashMap<String, Vec<String>>,
) -> HashMap<String, HashSet<String>> {
    let mut adj: HashMap<String, HashSet<String>> = HashMap::new();
    for (node, targets) in outgoing {
        for t in targets {
            adj.entry(node.clone()).or_default().insert(t.clone());
            adj.entry(t.clone()).or_default().insert(node.clone());
        }
    }
    for (node, sources) in incoming {
        for s in sources {
            adj.entry(node.clone()).or_default().insert(s.clone());
            adj.entry(s.clone()).or_default().insert(node.clone());
        }
    }
    adj
}

fn edge_list(adj: &HashMap<String, HashSet<String>>) -> Vec<(String, String)> {
    let mut edges = Vec::new();
    for (a, neighbors) in adj {
        for b in neighbors {
            if a < b {
                edges.push((a.clone(), b.clone()));
            }
        }
    }
    edges
}

pub fn traverse(
    outgoing: &HashMap<String, Vec<String>>,
    incoming: &HashMap<String, Vec<String>>,
    start: &str,
    depth: usize,
) -> Vec<(String, usize, Vec<String>)> {
    let adj = to_undirected(outgoing, incoming);
    if !adj.contains_key(start) && !outgoing.contains_key(start) && !incoming.contains_key(start) {
        return Vec::new();
    }
    let mut dist = HashMap::new();
    let mut queue = VecDeque::new();
    dist.insert(start.to_string(), 0usize);
    queue.push_back(start.to_string());
    while let Some(node) = queue.pop_front() {
        let d = dist[&node];
        if d >= depth {
            continue;
        }
        if let Some(neighbors) = adj.get(&node) {
            let mut nbrs: Vec<_> = neighbors.iter().cloned().collect();
            nbrs.sort();
            for n in nbrs {
                dist.entry(n.clone()).or_insert_with(|| {
                    queue.push_back(n.clone());
                    d + 1
                });
            }
        }
    }
    let mut rows: Vec<_> = dist
        .into_iter()
        .map(|(name, d)| {
            let mut neighbors: Vec<String> = adj
                .get(&name)
                .map(|s| s.iter().cloned().collect())
                .unwrap_or_default();
            neighbors.sort();
            (name, d, neighbors)
        })
        .collect();
    rows.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    rows
}

pub fn shortest_path(
    outgoing: &HashMap<String, Vec<String>>,
    incoming: &HashMap<String, Vec<String>>,
    from: &str,
    to: &str,
) -> Option<Vec<String>> {
    if from == to {
        return Some(vec![from.to_string()]);
    }
    let adj = to_undirected(outgoing, incoming);
    let mut prev: HashMap<String, String> = HashMap::new();
    let mut queue = VecDeque::new();
    let mut seen = HashSet::new();
    queue.push_back(from.to_string());
    seen.insert(from.to_string());
    while let Some(node) = queue.pop_front() {
        let mut neighbors: Vec<String> = adj
            .get(&node)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default();
        neighbors.sort();
        for n in neighbors {
            if seen.insert(n.clone()) {
                prev.insert(n.clone(), node.clone());
                if n == to {
                    let mut path = vec![to.to_string()];
                    let mut cur = to.to_string();
                    while let Some(p) = prev.get(&cur) {
                        path.push(p.clone());
                        cur = p.clone();
                    }
                    path.reverse();
                    return Some(path);
                }
                queue.push_back(n);
            }
        }
    }
    None
}

pub fn detect_communities(
    outgoing: &HashMap<String, Vec<String>>,
    incoming: &HashMap<String, Vec<String>>,
) -> (HashMap<String, usize>, Vec<Vec<String>>) {
    let adj = to_undirected(outgoing, incoming);
    let edges = edge_list(&adj);
    let m = edges.len() as f64;

    if m == 0.0 {
        let mut community_of = HashMap::new();
        let mut communities = Vec::new();
        let mut nodes: Vec<_> = adj.keys().cloned().collect();
        nodes.sort();
        for (i, node) in nodes.into_iter().enumerate() {
            community_of.insert(node.clone(), i);
            communities.push(vec![node]);
        }
        return (community_of, communities);
    }

    let degree: HashMap<&String, f64> = adj.iter().map(|(k, v)| (k, v.len() as f64)).collect();
    let mut nodes: Vec<String> = adj.keys().cloned().collect();
    nodes.sort();
    let mut community_of: HashMap<String, usize> = HashMap::new();
    for (i, node) in nodes.iter().enumerate() {
        community_of.insert(node.clone(), i);
    }

    for _ in 0..20 {
        let mut changed = false;
        for node in &nodes {
            let node_comm = community_of[node];
            let node_deg = degree.get(node).copied().unwrap_or(0.0);
            let Some(neighbors) = adj.get(node) else {
                continue;
            };

            let mut comm_edges: HashMap<usize, f64> = HashMap::new();
            for neighbor in neighbors {
                let nc = community_of[neighbor];
                *comm_edges.entry(nc).or_default() += 1.0;
            }

            let mut comm_degree_sum: HashMap<usize, f64> = HashMap::new();
            for (n, &c) in &community_of {
                *comm_degree_sum.entry(c).or_default() += degree.get(n).copied().unwrap_or(0.0);
            }

            let mut best_comm = node_comm;
            let mut best_gain = 0.0_f64;
            let mut candidates: Vec<(usize, f64)> =
                comm_edges.iter().map(|(&c, &e)| (c, e)).collect();
            candidates.sort_by_key(|(c, _)| *c);
            for (candidate_comm, edges_to_comm) in candidates {
                if candidate_comm == node_comm {
                    continue;
                }
                let sigma_tot = comm_degree_sum.get(&candidate_comm).copied().unwrap_or(0.0);
                let gain = edges_to_comm / m - (sigma_tot * node_deg) / (2.0 * m * m);
                if gain > best_gain {
                    best_gain = gain;
                    best_comm = candidate_comm;
                }
            }
            if best_comm != node_comm {
                community_of.insert(node.clone(), best_comm);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    let mut id_map: HashMap<usize, usize> = HashMap::new();
    let mut next_id = 0;
    for &c in community_of.values() {
        id_map.entry(c).or_insert_with(|| {
            let id = next_id;
            next_id += 1;
            id
        });
    }
    for v in community_of.values_mut() {
        *v = id_map[v];
    }

    let mut members: HashMap<usize, Vec<String>> = HashMap::new();
    for (node, &comm) in &community_of {
        members.entry(comm).or_default().push(node.clone());
    }

    let small: Vec<usize> = members
        .iter()
        .filter(|(_, m)| m.len() < 3)
        .map(|(&c, _)| c)
        .collect();
    for small_comm in small {
        let small_nodes = members.remove(&small_comm).unwrap_or_default();
        for node in &small_nodes {
            let mut neighbor_comm_counts: HashMap<usize, usize> = HashMap::new();
            if let Some(neighbors) = adj.get(node) {
                for n in neighbors {
                    let nc = community_of[n];
                    if nc != small_comm {
                        *neighbor_comm_counts.entry(nc).or_default() += 1;
                    }
                }
            }
            if let Some((&best_comm, _)) = neighbor_comm_counts.iter().max_by_key(|(_, &v)| v) {
                community_of.insert(node.clone(), best_comm);
                members.entry(best_comm).or_default().push(node.clone());
            }
        }
    }

    let mut final_members: HashMap<usize, Vec<String>> = HashMap::new();
    for (node, &comm) in &community_of {
        final_members.entry(comm).or_default().push(node.clone());
    }
    let mut communities: Vec<Vec<String>> = final_members.into_values().collect();
    for c in &mut communities {
        c.sort();
    }
    communities.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a[0].cmp(&b[0])));

    let mut sorted_map = HashMap::new();
    for (i, comm) in communities.iter().enumerate() {
        for node in comm {
            sorted_map.insert(node.clone(), i);
        }
    }
    (sorted_map, communities)
}

pub fn betweenness_centrality(
    outgoing: &HashMap<String, Vec<String>>,
    incoming: &HashMap<String, Vec<String>>,
) -> HashMap<String, f64> {
    let adj = to_undirected(outgoing, incoming);
    let mut nodes: Vec<String> = adj.keys().cloned().collect();
    nodes.sort();
    let n = nodes.len();
    if n == 0 {
        return HashMap::new();
    }

    let mut centrality: HashMap<String, f64> = nodes.iter().map(|n| (n.clone(), 0.0)).collect();
    let sources: Vec<&String> = if n <= 500 {
        nodes.iter().collect()
    } else {
        let step = (n / 100).max(1);
        nodes.iter().step_by(step).take(100).collect()
    };

    for source in sources {
        let mut stack = Vec::new();
        let mut predecessors: HashMap<String, Vec<String>> = HashMap::new();
        let mut sigma: HashMap<String, f64> = HashMap::new();
        let mut dist: HashMap<String, i64> = HashMap::new();
        let mut delta: HashMap<String, f64> = HashMap::new();
        for node in &nodes {
            sigma.insert(node.clone(), 0.0);
            dist.insert(node.clone(), -1);
            delta.insert(node.clone(), 0.0);
        }
        sigma.insert(source.clone(), 1.0);
        dist.insert(source.clone(), 0);
        let mut queue = VecDeque::new();
        queue.push_back(source.clone());

        while let Some(v) = queue.pop_front() {
            stack.push(v.clone());
            let v_dist = dist[&v];
            if let Some(neighbors) = adj.get(&v) {
                for w in neighbors {
                    if dist[w] < 0 {
                        dist.insert(w.clone(), v_dist + 1);
                        queue.push_back(w.clone());
                    }
                    if dist[w] == v_dist + 1 {
                        *sigma.get_mut(w).unwrap() += sigma[&v];
                        predecessors.entry(w.clone()).or_default().push(v.clone());
                    }
                }
            }
        }

        while let Some(w) = stack.pop() {
            if let Some(preds) = predecessors.get(&w) {
                for v in preds {
                    let contribution = (sigma[v] / sigma[&w]) * (1.0 + delta[&w]);
                    *delta.get_mut(v).unwrap() += contribution;
                }
            }
            if &w != source {
                *centrality.get_mut(&w).unwrap() += delta[&w];
            }
        }
    }

    let max_val = centrality.values().cloned().fold(0.0_f64, f64::max);
    if max_val > 0.0 {
        for v in centrality.values_mut() {
            *v /= max_val;
        }
    }
    centrality
}

pub fn pagerank(outgoing: &HashMap<String, Vec<String>>) -> HashMap<String, f64> {
    let mut all_nodes: HashSet<String> = HashSet::new();
    for (k, vs) in outgoing {
        all_nodes.insert(k.clone());
        for v in vs {
            all_nodes.insert(v.clone());
        }
    }
    let mut nodes: Vec<String> = all_nodes.into_iter().collect();
    nodes.sort();
    let n_nodes = nodes.len();
    if n_nodes == 0 {
        return HashMap::new();
    }
    let d = 0.85;
    let init = 1.0 / n_nodes as f64;
    let mut rank: HashMap<String, f64> = nodes.iter().map(|node| (node.clone(), init)).collect();
    for _ in 0..50 {
        let mut new_rank: HashMap<String, f64> = nodes
            .iter()
            .map(|node| (node.clone(), (1.0 - d) / n_nodes as f64))
            .collect();
        for (node, targets) in outgoing {
            if targets.is_empty() {
                continue;
            }
            let share = rank.get(node).copied().unwrap_or(0.0) * d / targets.len() as f64;
            for target in targets {
                *new_rank.entry(target.clone()).or_default() += share;
            }
        }
        rank = new_rank;
    }
    let max_val = rank.values().cloned().fold(0.0_f64, f64::max);
    if max_val > 0.0 {
        for v in rank.values_mut() {
            *v /= max_val;
        }
    }
    rank
}

pub struct GodNode {
    pub name: String,
    pub score: f64,
    pub degree: usize,
    pub betweenness: f64,
    pub pagerank: f64,
}

pub fn god_nodes(
    outgoing: &HashMap<String, Vec<String>>,
    incoming: &HashMap<String, Vec<String>>,
    top_n: usize,
) -> Vec<GodNode> {
    let adj = to_undirected(outgoing, incoming);
    let bc = betweenness_centrality(outgoing, incoming);
    let pr = pagerank(outgoing);
    let max_degree = adj.values().map(|v| v.len()).max().unwrap_or(1).max(1) as f64;
    let mut nodes: Vec<GodNode> = adj
        .iter()
        .map(|(name, neighbors)| {
            let degree = neighbors.len();
            let betweenness = bc.get(name).copied().unwrap_or(0.0);
            let pr_score = pr.get(name).copied().unwrap_or(0.0);
            let score = 0.4 * (degree as f64 / max_degree) + 0.3 * betweenness + 0.3 * pr_score;
            GodNode {
                name: name.clone(),
                score,
                degree,
                betweenness,
                pagerank: pr_score,
            }
        })
        .collect();
    nodes.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    nodes.truncate(top_n);
    nodes
}

pub fn surprising_connections(
    outgoing: &HashMap<String, Vec<String>>,
    community_of: &HashMap<String, usize>,
    bc: &HashMap<String, f64>,
    top_n: usize,
) -> Vec<(String, String, f64)> {
    let mut cross_edges = Vec::new();
    for (source, targets) in outgoing {
        let source_comm = community_of.get(source).copied().unwrap_or(usize::MAX);
        for target in targets {
            let target_comm = community_of.get(target).copied().unwrap_or(usize::MAX);
            if source_comm != target_comm {
                let score =
                    bc.get(source).copied().unwrap_or(0.0) + bc.get(target).copied().unwrap_or(0.0);
                cross_edges.push((source.clone(), target.clone(), score));
            }
        }
    }
    cross_edges.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    cross_edges.truncate(top_n);
    cross_edges
}

pub fn is_orphan_stem(
    stem: &str,
    outgoing: &HashMap<String, Vec<String>>,
    incoming: &HashMap<String, Vec<String>>,
) -> bool {
    let out = outgoing.get(stem).map_or(0, |v| v.len());
    let inc = incoming.get(stem).map_or(0, |v| v.len());
    out == 0 && inc == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> (HashMap<String, Vec<String>>, HashMap<String, Vec<String>>) {
        let mut outgoing = HashMap::new();
        outgoing.insert("A".into(), vec!["B".into(), "C".into()]);
        outgoing.insert("B".into(), vec!["C".into()]);
        outgoing.insert("C".into(), vec!["D".into()]);
        outgoing.insert("D".into(), vec![]);
        outgoing.insert("Orphan".into(), vec![]);
        let mut incoming: HashMap<String, Vec<String>> = HashMap::new();
        incoming.insert("B".into(), vec!["A".into()]);
        incoming.insert("C".into(), vec!["A".into(), "B".into()]);
        incoming.insert("D".into(), vec!["C".into()]);
        incoming.insert("A".into(), vec![]);
        incoming.insert("Orphan".into(), vec![]);
        (outgoing, incoming)
    }

    #[test]
    fn bfs_and_shortest_path() {
        let (out, inc) = sample();
        let rows = traverse(&out, &inc, "A", 2);
        assert!(rows.iter().any(|(n, d, _)| n == "D" && *d == 2));
        assert_eq!(
            shortest_path(&out, &inc, "A", "D").unwrap(),
            vec!["A", "C", "D"]
        );
        assert!(shortest_path(&out, &inc, "A", "Orphan").is_none());
    }

    #[test]
    fn orphan_requires_no_edges_either_way() {
        let (out, inc) = sample();
        assert!(!is_orphan_stem("A", &out, &inc));
        assert!(is_orphan_stem("Orphan", &out, &inc));
        assert!(is_orphan_stem("missing", &out, &inc));
    }
}
