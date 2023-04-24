use crate::Graph;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PlayHead {
    pub upper: u8,
    pub lower: u8,
    pub div: u8,
    pub samples_per_quarter: f64,
    pub samples_from_last_bar: f64,
}

pub struct GraphPlayer {
    pub graph: Graph,
}

impl GraphPlayer {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }
}
