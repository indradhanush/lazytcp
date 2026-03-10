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

#[cfg(test)]
mod tests {
    use super::FilterDimension;

    #[test]
    fn filter_dimension_all_contains_v1_dimensions_in_order() {
        assert_eq!(
            FilterDimension::ALL,
            [
                FilterDimension::Host,
                FilterDimension::Source,
                FilterDimension::Destination,
                FilterDimension::Port,
                FilterDimension::Protocol,
            ]
        );
    }

    #[test]
    fn filter_dimension_as_str_maps_expected_keywords() {
        assert_eq!(FilterDimension::Host.as_str(), "host");
        assert_eq!(FilterDimension::Source.as_str(), "source");
        assert_eq!(FilterDimension::Destination.as_str(), "destination");
        assert_eq!(FilterDimension::Port.as_str(), "port");
        assert_eq!(FilterDimension::Protocol.as_str(), "protocol");
    }
}
