use std::{collections::HashMap, path::Path};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct NetlistV1 {
    pub chip: String,
    #[serde(default)]
    pub devices: Devices,
    #[serde(default)]
    pub signals: Vec<Signal>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Devices {
    #[serde(default)]
    pub transistors: Vec<Transistor>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Transistor {
    #[serde(default)]
    pub id: Option<u32>,
    #[serde(default)]
    pub kind: Option<String>,
    pub gate_node: i32,
    pub a_node: i32,
    pub b_node: i32,
    #[serde(default)]
    pub bbox: Option<BBoxV1>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BBoxV1 {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Signal {
    pub name: String,
    #[serde(default)]
    pub layout_node: Option<i32>,
    #[serde(default)]
    pub evidence: Evidence,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Evidence {
    #[serde(default)]
    pub anchor: bool,
}

pub fn load_netlist_v1(path: impl AsRef<Path>) -> Result<NetlistV1, std::io::Error> {
    let s = std::fs::read_to_string(path)?;
    serde_json::from_str(&s).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

pub fn transistor_incidence(net: &NetlistV1) -> HashMap<i32, u32> {
    let mut out: HashMap<i32, u32> = HashMap::new();
    for t in &net.devices.transistors {
        *out.entry(t.gate_node).or_insert(0) += 1;
        *out.entry(t.a_node).or_insert(0) += 1;
        *out.entry(t.b_node).or_insert(0) += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_4004_netlist_v1() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .expect("repo root");
        let path = repo_root.join("docs/evidence/netlists_v1/4004_netlist_v1.json");
        let net = load_netlist_v1(&path).expect("load netlist_v1");
        assert_eq!(net.chip, "4004");
        assert!(!net.devices.transistors.is_empty());
        let inc = transistor_incidence(&net);
        let anchor_nodes: Vec<i32> = net
            .signals
            .iter()
            .filter(|s| s.evidence.anchor)
            .filter_map(|s| s.layout_node)
            .collect();
        assert!(!anchor_nodes.is_empty());
        for n in anchor_nodes {
            assert!(
                inc.get(&n).copied().unwrap_or(0) > 0,
                "anchor node had zero incidence: {n}"
            );
        }
    }
}
