#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterDimension {
    Host,
    Source,
    Destination,
    Port,
    Protocol,
}

impl FilterDimension {
    pub const ALL: [FilterDimension; 5] = [
        FilterDimension::Host,
        FilterDimension::Source,
        FilterDimension::Destination,
        FilterDimension::Port,
        FilterDimension::Protocol,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            FilterDimension::Host => "host",
            FilterDimension::Source => "source",
            FilterDimension::Destination => "destination",
            FilterDimension::Port => "port",
            FilterDimension::Protocol => "protocol",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PacketSummary {
    pub timestamp: String,
    pub source: String,
    pub destination: String,
    pub protocol: String,
    pub length: usize,
    pub summary: String,
}
