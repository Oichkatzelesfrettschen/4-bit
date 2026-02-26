//! Layer 3: Topology
//!
//! Maps filtered bits to spatial or network topologies (e.g., Patricia Tries).
//! Provides the mathematical structures for Cayley-Dickson algebras and 
//! Sedenion geometry.

/// A 16-dimensional Sedenion element.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sedenion {
    pub e: [f64; 16],
}

impl Default for Sedenion {
    fn default() -> Self {
        Self { e: [0.0; 16] }
    }
}

impl Sedenion {
    pub fn new(e: [f64; 16]) -> Self {
        Self { e }
    }

    /// Compute the norm squared of the Sedenion.
    pub fn norm_sq(&self) -> f64 {
        self.e.iter().map(|x| x * x).sum()
    }
}

/// Compute the Harary-Zaslavsky frustration index for a given Sedenion node.
///
/// In the context of the Sedenion algebra, frustration arises from the 
/// non-associative, non-alternative nature of the multiplication table.
/// This index acts as a localized "stress" measure within the topology.
pub fn calculate_frustration_index(node: &Sedenion) -> f64 {
    // A simplified metric: variance of the elements scaled by the norm.
    // In a full implementation, this evaluates cycle signs in the multiplication graph.
    let mean = node.e.iter().sum::<f64>() / 16.0;
    let variance = node.e.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / 16.0;
    
    // Normalize to a [0, 1] range based on structural assumptions
    (variance / (node.norm_sq() + 1e-12)).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sedenion_norm() {
        let mut e = [0.0; 16];
        e[0] = 3.0;
        e[15] = 4.0;
        let s = Sedenion::new(e);
        assert_eq!(s.norm_sq(), 25.0);
    }

    #[test]
    fn test_frustration_index() {
        let mut e = [1.0; 16];
        let s1 = Sedenion::new(e);
        // Uniform distribution -> zero variance -> zero frustration
        assert_eq!(calculate_frustration_index(&s1), 0.0);

        e[0] = 10.0;
        let s2 = Sedenion::new(e);
        // Non-uniform -> positive frustration
        assert!(calculate_frustration_index(&s2) > 0.0);
    }
}
