use std::collections::HashMap;

use crate::graph::GodNode;

#[allow(clippy::too_many_arguments)]
pub fn generate_report(
    vault_name: &str,
    total_nodes: usize,
    total_edges: usize,
    communities: &[Vec<String>],
    god_nodes: &[GodNode],
    surprising: &[(String, String, f64)],
    orphan_count: usize,
    community_of: &HashMap<String, usize>,
) -> String {
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    let mut report = String::new();
    report.push_str(&format!(
        "---\ntitle: \"Graph Report: {vault_name}\"\ndate: {date}\ntype: report\n---\n\n"
    ));
    report.push_str(&format!("# Graph Report: {vault_name}\n\n"));
    report.push_str(&format!(
        "**Generated:** {date} | **Notes:** {total_nodes} | **Links:** {total_edges} | **Communities:** {} | **Orphans:** {orphan_count}\n\n",
        communities.len()
    ));

    report.push_str("## God nodes\n\n");
    report.push_str("Structurally important notes (degree + betweenness + PageRank).\n\n");
    report.push_str("| Rank | Note | Score | Connections | Betweenness | PageRank |\n");
    report.push_str("|------|------|-------|-------------|-------------|----------|\n");
    for (i, gn) in god_nodes.iter().enumerate() {
        report.push_str(&format!(
            "| {} | [[{}]] | {:.2} | {} | {:.2} | {:.2} |\n",
            i + 1,
            gn.name,
            gn.score,
            gn.degree,
            gn.betweenness,
            gn.pagerank
        ));
    }
    report.push('\n');

    report.push_str("## Communities\n\n");
    for (i, members) in communities.iter().enumerate() {
        let label = members.first().cloned().unwrap_or_default();
        report.push_str(&format!(
            "### Community {} — [[{}]] ({} notes)\n\n",
            i + 1,
            label,
            members.len()
        ));
        let display: Vec<String> = members
            .iter()
            .take(15)
            .map(|m| format!("[[{m}]]"))
            .collect();
        report.push_str(&display.join(", "));
        if members.len() > 15 {
            report.push_str(&format!(" ... and {} more", members.len() - 15));
        }
        report.push_str("\n\n");
    }

    if !surprising.is_empty() {
        report.push_str("## Surprising connections\n\n");
        for (source, target, score) in surprising {
            let s_comm = community_of.get(source).copied().unwrap_or(0);
            let t_comm = community_of.get(target).copied().unwrap_or(0);
            report.push_str(&format!(
                "- [[{source}]] (community {}) → [[{target}]] (community {}) — bridge score: {score:.2}\n",
                s_comm + 1,
                t_comm + 1
            ));
        }
        report.push('\n');
    }

    report.push_str("## Suggested questions\n\n");
    if god_nodes.len() >= 2 {
        report.push_str(&format!(
            "1. How does [[{}]] connect to [[{}]]?\n",
            god_nodes[0].name, god_nodes[1].name
        ));
    }
    if orphan_count > 0 {
        report.push_str(&format!(
            "2. Which of the {orphan_count} orphan notes should be linked into the wiki?\n"
        ));
    }
    report.push_str("3. What contradictions or stale claims would a lint pass surface?\n");
    report
}
