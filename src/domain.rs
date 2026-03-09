#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PacketSummary {
    pub timestamp: String,
    pub source: String,
    pub destination: String,
    pub protocol: String,
    pub length: usize,
    pub summary: String,
}

impl PacketSummary {
    pub fn demo_rows() -> Vec<Self> {
        vec![
            Self {
                timestamp: "12:00:01.123".to_string(),
                source: "10.0.0.12:51544".to_string(),
                destination: "1.1.1.1:443".to_string(),
                protocol: "TCP".to_string(),
                length: 74,
                summary: "Client Hello".to_string(),
            },
            Self {
                timestamp: "12:00:01.221".to_string(),
                source: "1.1.1.1:443".to_string(),
                destination: "10.0.0.12:51544".to_string(),
                protocol: "TCP".to_string(),
                length: 1514,
                summary: "Server Hello + Certificate".to_string(),
            },
            Self {
                timestamp: "12:00:02.022".to_string(),
                source: "10.0.0.12:34211".to_string(),
                destination: "8.8.8.8:53".to_string(),
                protocol: "UDP".to_string(),
                length: 92,
                summary: "DNS A query api.internal".to_string(),
            },
            Self {
                timestamp: "12:00:02.104".to_string(),
                source: "8.8.8.8:53".to_string(),
                destination: "10.0.0.12:34211".to_string(),
                protocol: "UDP".to_string(),
                length: 108,
                summary: "DNS A response 10.2.0.18".to_string(),
            },
            Self {
                timestamp: "12:00:03.011".to_string(),
                source: "10.0.0.12".to_string(),
                destination: "10.0.0.1".to_string(),
                protocol: "ICMP".to_string(),
                length: 98,
                summary: "Echo request".to_string(),
            },
            Self {
                timestamp: "12:00:03.092".to_string(),
                source: "10.0.0.1".to_string(),
                destination: "10.0.0.12".to_string(),
                protocol: "ICMP".to_string(),
                length: 98,
                summary: "Echo reply".to_string(),
            },
        ]
    }
}
