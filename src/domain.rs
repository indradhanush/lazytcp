#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PacketSummary {
    pub timestamp: String,
    pub source: String,
    pub destination: String,
    pub protocol: String,
    pub length: usize,
    pub summary: String,
}
