#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterDimension {
    Host,
    Source,
    Destination,
    Port,
    SourcePort,
    DestinationPort,
    Protocol,
    TcpFlags,
    IpVersion,
    TrafficClass,
    IcmpType,
}

impl FilterDimension {
    pub const ALL: [FilterDimension; 11] = [
        FilterDimension::Host,
        FilterDimension::Source,
        FilterDimension::Destination,
        FilterDimension::Port,
        FilterDimension::SourcePort,
        FilterDimension::DestinationPort,
        FilterDimension::Protocol,
        FilterDimension::TcpFlags,
        FilterDimension::IpVersion,
        FilterDimension::TrafficClass,
        FilterDimension::IcmpType,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            FilterDimension::Host => "host",
            FilterDimension::Source => "source",
            FilterDimension::Destination => "destination",
            FilterDimension::Port => "port",
            FilterDimension::SourcePort => "src port",
            FilterDimension::DestinationPort => "dst port",
            FilterDimension::Protocol => "protocol",
            FilterDimension::TcpFlags => "tcp flags",
            FilterDimension::IpVersion => "ip version",
            FilterDimension::TrafficClass => "traffic class",
            FilterDimension::IcmpType => "icmp type",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TcpFlags {
    pub ns: bool,
    pub cwr: bool,
    pub ece: bool,
    pub urg: bool,
    pub ack: bool,
    pub psh: bool,
    pub rst: bool,
    pub syn: bool,
    pub fin: bool,
    pub raw: String,
}

impl TcpFlags {
    fn from_tcpdump(value: Option<&str>) -> Self {
        let mut flags = Self {
            ns: false,
            cwr: false,
            ece: false,
            urg: false,
            ack: false,
            psh: false,
            rst: false,
            syn: false,
            fin: false,
            raw: value.unwrap_or("").to_string(),
        };

        for ch in flags.raw.chars() {
            match ch {
                'N' => flags.ns = true,
                'W' => flags.cwr = true,
                'E' => flags.ece = true,
                'U' => flags.urg = true,
                '.' => flags.ack = true,
                'P' => flags.psh = true,
                'R' => flags.rst = true,
                'S' => flags.syn = true,
                'F' => flags.fin = true,
                _ => {}
            }
        }

        flags
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TcpPacketDetails {
    pub source_port: Option<u16>,
    pub destination_port: Option<u16>,
    pub sequence_number: Option<String>,
    pub acknowledgement_number: Option<String>,
    pub data_offset_words: Option<u8>,
    pub reserved_bits: Option<String>,
    pub flags: TcpFlags,
    pub window_size: Option<u32>,
    pub checksum: Option<String>,
    pub urgent_pointer: Option<u16>,
    pub options: Option<String>,
    pub payload_length: usize,
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

pub fn tcp_packet_details(packet: &PacketSummary) -> Option<TcpPacketDetails> {
    if !packet.protocol.eq_ignore_ascii_case("TCP") {
        return None;
    }

    let flags_raw = extract_bracketed_field(&packet.summary, "Flags [", ']');

    Some(TcpPacketDetails {
        source_port: parse_endpoint_port(&packet.source),
        destination_port: parse_endpoint_port(&packet.destination),
        sequence_number: extract_comma_field(&packet.summary, "seq "),
        acknowledgement_number: extract_comma_field(&packet.summary, "ack "),
        data_offset_words: extract_numeric_field_u8(&packet.summary, &["offset ", "doff "]),
        reserved_bits: extract_comma_field(&packet.summary, "reserved "),
        flags: TcpFlags::from_tcpdump(flags_raw.as_deref()),
        window_size: extract_numeric_field_u32(&packet.summary, "win "),
        checksum: extract_comma_field(&packet.summary, "cksum "),
        urgent_pointer: extract_numeric_field_u16(&packet.summary, "urg "),
        options: extract_bracketed_field(&packet.summary, "options [", ']'),
        payload_length: packet.length,
    })
}

fn parse_endpoint_port(endpoint: &str) -> Option<u16> {
    let (host, port) = endpoint.rsplit_once('.')?;
    if host.is_empty() || !port.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    port.parse::<u16>().ok()
}

fn extract_comma_field(summary: &str, key: &str) -> Option<String> {
    let start = summary.find(key)? + key.len();
    let remainder = &summary[start..];
    let end = remainder.find(',').unwrap_or(remainder.len());
    let value = remainder[..end].trim();

    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn extract_bracketed_field(summary: &str, prefix: &str, closing: char) -> Option<String> {
    let start = summary.find(prefix)? + prefix.len();
    let remainder = &summary[start..];
    let end = remainder.find(closing)?;
    let value = remainder[..end].trim();

    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn extract_numeric_field_u8(summary: &str, keys: &[&str]) -> Option<u8> {
    for key in keys {
        if let Some(value) = extract_comma_field(summary, key) {
            if let Ok(parsed) = value.parse::<u8>() {
                return Some(parsed);
            }
        }
    }
    None
}

fn extract_numeric_field_u16(summary: &str, key: &str) -> Option<u16> {
    extract_comma_field(summary, key)?.parse::<u16>().ok()
}

fn extract_numeric_field_u32(summary: &str, key: &str) -> Option<u32> {
    extract_comma_field(summary, key)?.parse::<u32>().ok()
}

#[cfg(test)]
mod tests {
    use super::{tcp_packet_details, FilterDimension, PacketSummary, TcpFlags};

    fn sample_tcp_packet(summary: &str) -> PacketSummary {
        PacketSummary {
            timestamp: "1970-01-01 00:00:01.001000".to_string(),
            source: "10.0.0.12.51544".to_string(),
            destination: "1.1.1.1.443".to_string(),
            protocol: "TCP".to_string(),
            length: 0,
            summary: summary.to_string(),
        }
    }

    #[test]
    fn filter_dimension_all_contains_v1_dimensions_in_order() {
        assert_eq!(
            FilterDimension::ALL,
            [
                FilterDimension::Host,
                FilterDimension::Source,
                FilterDimension::Destination,
                FilterDimension::Port,
                FilterDimension::SourcePort,
                FilterDimension::DestinationPort,
                FilterDimension::Protocol,
                FilterDimension::TcpFlags,
                FilterDimension::IpVersion,
                FilterDimension::TrafficClass,
                FilterDimension::IcmpType,
            ]
        );
    }

    #[test]
    fn filter_dimension_as_str_maps_expected_keywords() {
        assert_eq!(FilterDimension::Host.as_str(), "host");
        assert_eq!(FilterDimension::Source.as_str(), "source");
        assert_eq!(FilterDimension::Destination.as_str(), "destination");
        assert_eq!(FilterDimension::Port.as_str(), "port");
        assert_eq!(FilterDimension::SourcePort.as_str(), "src port");
        assert_eq!(FilterDimension::DestinationPort.as_str(), "dst port");
        assert_eq!(FilterDimension::Protocol.as_str(), "protocol");
        assert_eq!(FilterDimension::TcpFlags.as_str(), "tcp flags");
        assert_eq!(FilterDimension::IpVersion.as_str(), "ip version");
        assert_eq!(FilterDimension::TrafficClass.as_str(), "traffic class");
        assert_eq!(FilterDimension::IcmpType.as_str(), "icmp type");
    }

    #[test]
    fn tcp_packet_details_extracts_ports_and_summary_fields() {
        let packet = sample_tcp_packet(
            "Flags [S], seq 12345, win 65535, options [mss 1460,sackOK], length 0",
        );

        let details = tcp_packet_details(&packet).expect("TCP packet should parse");
        assert_eq!(details.source_port, Some(51544));
        assert_eq!(details.destination_port, Some(443));
        assert_eq!(details.sequence_number.as_deref(), Some("12345"));
        assert_eq!(details.window_size, Some(65535));
        assert_eq!(details.options.as_deref(), Some("mss 1460,sackOK"));
        assert!(details.flags.syn);
        assert!(!details.flags.ack);
    }

    #[test]
    fn tcp_flags_decode_from_tcpdump_flag_string() {
        let flags = TcpFlags::from_tcpdump(Some("S."));

        assert!(flags.syn);
        assert!(flags.ack);
        assert!(!flags.fin);
        assert_eq!(flags.raw, "S.");
    }

    #[test]
    fn tcp_packet_details_extracts_ack_and_urg_fields_when_present() {
        let packet =
            sample_tcp_packet("Flags [P.], ack 222, win 1024, urg 3, cksum 0x1234, length 12");

        let details = tcp_packet_details(&packet).expect("TCP packet should parse");
        assert_eq!(details.acknowledgement_number.as_deref(), Some("222"));
        assert_eq!(details.urgent_pointer, Some(3));
        assert_eq!(details.checksum.as_deref(), Some("0x1234"));
        assert_eq!(details.payload_length, 0);
        assert!(details.flags.psh);
        assert!(details.flags.ack);
    }

    #[test]
    fn tcp_packet_details_returns_none_for_non_tcp_protocol() {
        let packet = PacketSummary {
            timestamp: "1970-01-01 00:00:02.002000".to_string(),
            source: "10.0.0.12.34211".to_string(),
            destination: "8.8.8.8.53".to_string(),
            protocol: "UDP".to_string(),
            length: 0,
            summary: "UDP, length 0".to_string(),
        };

        assert!(tcp_packet_details(&packet).is_none());
    }

    #[test]
    fn tcp_packet_details_tolerates_missing_optional_fields() {
        let packet = sample_tcp_packet("Flags [S], length 0");

        let details = tcp_packet_details(&packet).expect("TCP packet should parse");
        assert_eq!(details.sequence_number, None);
        assert_eq!(details.acknowledgement_number, None);
        assert_eq!(details.window_size, None);
        assert_eq!(details.options, None);
        assert!(details.flags.syn);
    }
}
