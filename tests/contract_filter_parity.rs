use std::collections::BTreeSet;
use std::error::Error;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use lazytcp::api::{TcpdumpApi, TcpdumpReadRequest, parse_tcpdump_stdout};
use lazytcp::app::App;
use lazytcp::domain::{FilterDimension, PacketSummary};

const ALL_TCP_FLAG_LABELS: [&str; 9] =
    ["NS", "CWR", "ECE", "URG", "ACK", "PSH", "RST", "SYN", "FIN"];

#[test]
fn filter_candidates_match_tcpdump_baseline_for_all_non_datetime_dimensions()
-> Result<(), Box<dyn Error>> {
    if !tcpdump_available() {
        eprintln!("skipping contract test: tcpdump is not installed");
        return Ok(());
    }

    let pcap_path = fixture_path("contract-filter-parity-candidates");
    write_filter_parity_fixture(&pcap_path)?;

    let result = (|| -> Result<(), Box<dyn Error>> {
        let packets = read_packets_via_api(&pcap_path)?;
        let tcpdump_packets = run_tcpdump_packets(&pcap_path, &[])?;
        assert_eq!(packet_rows(&packets), packet_rows(&tcpdump_packets));

        for dimension in FilterDimension::ALL {
            if dimension == FilterDimension::DateTime {
                continue;
            }

            let actual = popup_candidates_for_dimension(&packets, dimension);
            let expected = expected_candidates_for_dimension(&tcpdump_packets, dimension);
            assert_eq!(
                actual, expected,
                "candidate mismatch for dimension {dimension:?}"
            );
        }

        Ok(())
    })();

    let _ = fs::remove_file(&pcap_path);
    result
}

#[test]
fn single_value_filters_match_tcpdump_contract_for_all_non_datetime_dimensions()
-> Result<(), Box<dyn Error>> {
    if !tcpdump_available() {
        eprintln!("skipping contract test: tcpdump is not installed");
        return Ok(());
    }

    let pcap_path = fixture_path("contract-filter-parity-single");
    write_filter_parity_fixture(&pcap_path)?;

    let result = (|| -> Result<(), Box<dyn Error>> {
        let packets = read_packets_via_api(&pcap_path)?;
        let tcpdump_packets = run_tcpdump_packets(&pcap_path, &[])?;

        for dimension in FilterDimension::ALL {
            if dimension == FilterDimension::DateTime {
                continue;
            }

            let values = expected_candidates_for_dimension(&tcpdump_packets, dimension);
            for value in values {
                let actual = apply_single_select_filter(&packets, dimension, &value);
                let expected = expected_packets_for_single_value(
                    &pcap_path,
                    &tcpdump_packets,
                    dimension,
                    &value,
                )?;

                assert_eq!(
                    packet_rows(&actual),
                    packet_rows(&expected),
                    "single-select mismatch for dimension {dimension:?} and value {value}"
                );
            }
        }

        Ok(())
    })();

    let _ = fs::remove_file(&pcap_path);
    result
}

#[test]
fn date_time_range_filter_matches_tcpdump_timestamp_baseline() -> Result<(), Box<dyn Error>> {
    if !tcpdump_available() {
        eprintln!("skipping contract test: tcpdump is not installed");
        return Ok(());
    }

    let pcap_path = fixture_path("contract-filter-parity-date-time");
    write_filter_parity_fixture(&pcap_path)?;

    let result = (|| -> Result<(), Box<dyn Error>> {
        let packets = read_packets_via_api(&pcap_path)?;
        let tcpdump_packets = run_tcpdump_packets(&pcap_path, &[])?;

        let mut timestamps: Vec<String> = tcpdump_packets
            .iter()
            .map(|packet| packet.timestamp.clone())
            .collect();
        timestamps.sort();
        timestamps.dedup();
        assert!(
            timestamps.len() >= 3,
            "fixture must contain at least three timestamps"
        );

        let start = timestamps[1].as_str();
        let end = timestamps[timestamps.len() - 2].as_str();

        let actual_start_only = apply_date_time_range_filter(&packets, Some(start), None);
        let expected_start_only: Vec<PacketSummary> = tcpdump_packets
            .iter()
            .filter(|packet| packet.timestamp.as_str() >= start)
            .cloned()
            .collect();
        assert_eq!(
            packet_rows(&actual_start_only),
            packet_rows(&expected_start_only),
            "date-time start-only mismatch"
        );

        let actual_end_only = apply_date_time_range_filter(&packets, None, Some(end));
        let expected_end_only: Vec<PacketSummary> = tcpdump_packets
            .iter()
            .filter(|packet| packet.timestamp.as_str() <= end)
            .cloned()
            .collect();
        assert_eq!(
            packet_rows(&actual_end_only),
            packet_rows(&expected_end_only),
            "date-time end-only mismatch"
        );

        let actual_between = apply_date_time_range_filter(&packets, Some(start), Some(end));
        let expected_between: Vec<PacketSummary> = tcpdump_packets
            .iter()
            .filter(|packet| packet.timestamp.as_str() >= start && packet.timestamp.as_str() <= end)
            .cloned()
            .collect();
        assert_eq!(
            packet_rows(&actual_between),
            packet_rows(&expected_between),
            "date-time bounded range mismatch"
        );

        Ok(())
    })();

    let _ = fs::remove_file(&pcap_path);
    result
}

fn popup_candidates_for_dimension(
    packets: &[PacketSummary],
    dimension: FilterDimension,
) -> Vec<String> {
    let mut app = App::with_packets(packets.to_vec(), String::new());
    select_filter_dimension(&mut app, dimension);
    app.open_filter_popup();

    app.filter_popup_candidates()
        .map(|candidates| candidates.to_vec())
        .unwrap_or_default()
}

fn apply_single_select_filter(
    packets: &[PacketSummary],
    dimension: FilterDimension,
    value: &str,
) -> Vec<PacketSummary> {
    let mut app = App::with_packets(packets.to_vec(), String::new());
    select_filter_dimension(&mut app, dimension);
    app.open_filter_popup();

    let candidates = app
        .filter_popup_candidates()
        .map(|candidates| candidates.to_vec())
        .unwrap_or_default();

    let target_index = candidates
        .iter()
        .position(|candidate| candidate == value)
        .unwrap_or_else(|| panic!("missing candidate {value} for dimension {dimension:?}"));

    for _ in 0..target_index {
        app.move_down();
    }

    app.toggle_filter_popup_selection();
    app.confirm_filter_popup();

    app.packets().to_vec()
}

fn apply_date_time_range_filter(
    packets: &[PacketSummary],
    start: Option<&str>,
    end: Option<&str>,
) -> Vec<PacketSummary> {
    let mut app = App::with_packets(packets.to_vec(), String::new());
    select_filter_dimension(&mut app, FilterDimension::DateTime);
    app.open_filter_popup();
    app.clear_filter_popup_selection();

    if let Some(start) = start {
        for ch in start.chars() {
            app.filter_popup_insert_char(ch);
        }
    }

    if let Some(end) = end {
        app.filter_popup_switch_date_time_field();
        for ch in end.chars() {
            app.filter_popup_insert_char(ch);
        }
    }

    app.confirm_filter_popup();
    app.packets().to_vec()
}

fn expected_packets_for_single_value(
    pcap_path: &Path,
    baseline_packets: &[PacketSummary],
    dimension: FilterDimension,
    value: &str,
) -> Result<Vec<PacketSummary>, Box<dyn Error>> {
    if let Some(filter_args) = tcpdump_filter_args_for_single_value(dimension, value) {
        return run_tcpdump_packets(pcap_path, &filter_args);
    }

    Ok(baseline_packets
        .iter()
        .filter(|packet| packet_matches_value(packet, dimension, value))
        .cloned()
        .collect())
}

fn tcpdump_filter_args_for_single_value(
    dimension: FilterDimension,
    value: &str,
) -> Option<Vec<String>> {
    match dimension {
        FilterDimension::Host => Some(vec!["host".to_string(), value.to_string()]),
        FilterDimension::Source => Some(vec![
            "src".to_string(),
            "host".to_string(),
            value.to_string(),
        ]),
        FilterDimension::Destination => Some(vec![
            "dst".to_string(),
            "host".to_string(),
            value.to_string(),
        ]),
        FilterDimension::Interface => None,
        FilterDimension::Port => Some(vec!["port".to_string(), value.to_string()]),
        FilterDimension::SourcePort => Some(vec![
            "src".to_string(),
            "port".to_string(),
            value.to_string(),
        ]),
        FilterDimension::DestinationPort => Some(vec![
            "dst".to_string(),
            "port".to_string(),
            value.to_string(),
        ]),
        FilterDimension::Protocol => {
            let query = value.to_ascii_lowercase();
            match query.as_str() {
                "tcp" | "udp" | "icmp" | "icmp6" => Some(vec![query]),
                _ => None,
            }
        }
        FilterDimension::IpVersion => match value.to_ascii_lowercase().as_str() {
            "ipv4" => Some(vec!["ip".to_string()]),
            "ipv6" => Some(vec!["ip6".to_string()]),
            _ => None,
        },
        FilterDimension::TcpFlags
        | FilterDimension::DateTime
        | FilterDimension::TrafficClass
        | FilterDimension::IcmpType => None,
    }
}

fn expected_candidates_for_dimension(
    packets: &[PacketSummary],
    dimension: FilterDimension,
) -> Vec<String> {
    if dimension == FilterDimension::TcpFlags {
        return ALL_TCP_FLAG_LABELS
            .iter()
            .map(|label| (*label).to_string())
            .collect();
    }

    let mut candidates = BTreeSet::new();

    for packet in packets {
        match dimension {
            FilterDimension::Host => {
                candidates.insert(endpoint_host(&packet.source));
                candidates.insert(endpoint_host(&packet.destination));
            }
            FilterDimension::Source => {
                candidates.insert(endpoint_host(&packet.source));
            }
            FilterDimension::Destination => {
                candidates.insert(endpoint_host(&packet.destination));
            }
            FilterDimension::Interface => {
                if let Some(interface) = packet.interface.as_deref() {
                    candidates.insert(interface.to_ascii_lowercase());
                }
            }
            FilterDimension::Port => {
                if let Some(port) = endpoint_port(&packet.source) {
                    candidates.insert(port);
                }
                if let Some(port) = endpoint_port(&packet.destination) {
                    candidates.insert(port);
                }
            }
            FilterDimension::SourcePort => {
                if let Some(port) = endpoint_port(&packet.source) {
                    candidates.insert(port);
                }
            }
            FilterDimension::DestinationPort => {
                if let Some(port) = endpoint_port(&packet.destination) {
                    candidates.insert(port);
                }
            }
            FilterDimension::Protocol => {
                candidates.insert(packet.protocol.to_ascii_lowercase());
            }
            FilterDimension::TcpFlags => {}
            FilterDimension::IpVersion => {
                if let Some(version) = packet_ip_version(packet) {
                    candidates.insert(version.to_string());
                }
            }
            FilterDimension::DateTime => {
                candidates.insert(packet.timestamp.clone());
            }
            FilterDimension::TrafficClass => {
                if let Some(class) = packet_traffic_class(packet) {
                    candidates.insert(class.to_string());
                }
            }
            FilterDimension::IcmpType => {
                if let Some(icmp_type) = packet_icmp_type(packet) {
                    candidates.insert(icmp_type);
                }
            }
        }
    }

    let mut candidates: Vec<String> = candidates.into_iter().collect();
    if matches!(
        dimension,
        FilterDimension::Port | FilterDimension::SourcePort | FilterDimension::DestinationPort
    ) {
        candidates.sort_by(|left, right| {
            let left_port = left.parse::<u16>().ok();
            let right_port = right.parse::<u16>().ok();
            left_port.cmp(&right_port).then_with(|| left.cmp(right))
        });
    }

    candidates
}

fn packet_matches_value(packet: &PacketSummary, dimension: FilterDimension, value: &str) -> bool {
    let query = value.trim().to_ascii_lowercase();

    match dimension {
        FilterDimension::Host => {
            endpoint_host(&packet.source) == query || endpoint_host(&packet.destination) == query
        }
        FilterDimension::Source => endpoint_host(&packet.source) == query,
        FilterDimension::Destination => endpoint_host(&packet.destination) == query,
        FilterDimension::Interface => packet
            .interface
            .as_deref()
            .is_some_and(|interface| interface.eq_ignore_ascii_case(&query)),
        FilterDimension::Port => {
            endpoint_port(&packet.source).is_some_and(|port| port == query)
                || endpoint_port(&packet.destination).is_some_and(|port| port == query)
        }
        FilterDimension::SourcePort => {
            endpoint_port(&packet.source).is_some_and(|port| port == query)
        }
        FilterDimension::DestinationPort => {
            endpoint_port(&packet.destination).is_some_and(|port| port == query)
        }
        FilterDimension::Protocol => packet.protocol.to_ascii_lowercase() == query,
        FilterDimension::TcpFlags => {
            packet_matches_exact_tcp_flag_set(packet, std::iter::once(value))
        }
        FilterDimension::IpVersion => {
            packet_ip_version(packet).is_some_and(|version| version == query)
        }
        FilterDimension::DateTime => packet.timestamp == value.trim(),
        FilterDimension::TrafficClass => {
            packet_traffic_class(packet).is_some_and(|class| class == query)
        }
        FilterDimension::IcmpType => packet_icmp_type(packet)
            .as_deref()
            .is_some_and(|icmp_type| icmp_type == query),
    }
}

fn packet_matches_exact_tcp_flag_set<'a>(
    packet: &PacketSummary,
    selected_values: impl IntoIterator<Item = &'a str>,
) -> bool {
    let selected_flags: BTreeSet<String> = selected_values
        .into_iter()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();

    if selected_flags.is_empty() {
        return false;
    }

    let packet_flags: BTreeSet<&'static str> = tcp_flags_from_summary(&packet.summary)
        .into_iter()
        .collect();

    packet_flags.len() == selected_flags.len()
        && selected_flags
            .iter()
            .all(|selected_flag| packet_flags.contains(selected_flag.as_str()))
}

fn tcp_flags_from_summary(summary: &str) -> Vec<&'static str> {
    let Some(start) = summary.find("Flags [") else {
        return Vec::new();
    };
    let remainder = &summary[start + "Flags [".len()..];
    let Some(end) = remainder.find(']') else {
        return Vec::new();
    };

    let mut flags = Vec::new();
    for symbol in remainder[..end].chars() {
        let Some(label) = tcp_flag_label(symbol) else {
            continue;
        };
        flags.push(label);
    }

    flags
}

fn tcp_flag_label(symbol: char) -> Option<&'static str> {
    match symbol {
        'N' => Some("ns"),
        'W' => Some("cwr"),
        'E' => Some("ece"),
        'U' => Some("urg"),
        '.' => Some("ack"),
        'P' => Some("psh"),
        'R' => Some("rst"),
        'S' => Some("syn"),
        'F' => Some("fin"),
        _ => None,
    }
}

fn packet_ip_version(packet: &PacketSummary) -> Option<&'static str> {
    endpoint_ip_version(&packet.source).or_else(|| endpoint_ip_version(&packet.destination))
}

fn endpoint_ip_version(endpoint: &str) -> Option<&'static str> {
    let host = split_endpoint_host_port(endpoint)
        .map(|(host, _)| host)
        .unwrap_or(endpoint);

    if host.contains(':') {
        return Some("ipv6");
    }
    if is_ipv4_address(host) {
        return Some("ipv4");
    }

    None
}

fn packet_traffic_class(packet: &PacketSummary) -> Option<&'static str> {
    endpoint_traffic_class(&packet.destination)
}

fn endpoint_traffic_class(endpoint: &str) -> Option<&'static str> {
    let host = split_endpoint_host_port(endpoint)
        .map(|(host, _)| host)
        .unwrap_or(endpoint)
        .to_ascii_lowercase();

    if host.contains(':') {
        return Some(if host.starts_with("ff") {
            "multicast"
        } else {
            "unicast"
        });
    }

    if !is_ipv4_address(&host) {
        return None;
    }

    let octets: Vec<u8> = host
        .split('.')
        .filter_map(|part| part.parse::<u8>().ok())
        .collect();
    if octets.len() != 4 {
        return None;
    }

    if host == "255.255.255.255" || octets[3] == 255 {
        return Some("broadcast");
    }
    if (224..=239).contains(&octets[0]) {
        return Some("multicast");
    }

    Some("unicast")
}

fn packet_icmp_type(packet: &PacketSummary) -> Option<String> {
    if !packet.protocol.eq_ignore_ascii_case("ICMP")
        && !packet.protocol.eq_ignore_ascii_case("ICMP6")
    {
        return None;
    }

    icmp_type_from_summary(&packet.summary)
}

fn icmp_type_from_summary(summary: &str) -> Option<String> {
    let trimmed = summary.trim();
    let remainder = if let Some(rest) = trimmed.strip_prefix("ICMP6") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("ICMP") {
        rest
    } else {
        return None;
    };

    let normalized = remainder.trim_start_matches([',', ':', ' ']).trim_start();
    if normalized.is_empty() {
        return None;
    }

    let type_label = normalized
        .split(',')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some(type_label.to_ascii_lowercase())
}

fn endpoint_host(endpoint: &str) -> String {
    split_endpoint_host_port(endpoint)
        .map(|(host, _)| host.to_ascii_lowercase())
        .unwrap_or_else(|| endpoint.to_ascii_lowercase())
}

fn endpoint_port(endpoint: &str) -> Option<String> {
    split_endpoint_host_port(endpoint).map(|(_, port)| port.to_string())
}

fn split_endpoint_host_port(endpoint: &str) -> Option<(&str, &str)> {
    let (host, port) = endpoint.rsplit_once('.')?;
    if host.is_empty() || port.is_empty() || !port.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }

    if host.contains(':') || is_ipv4_address(host) {
        return Some((host, port));
    }

    None
}

fn is_ipv4_address(value: &str) -> bool {
    let mut parts = value.split('.');
    let mut count = 0;

    for part in parts.by_ref() {
        count += 1;
        if part.is_empty() || !part.chars().all(|ch| ch.is_ascii_digit()) {
            return false;
        }
        if part.parse::<u8>().is_err() {
            return false;
        }
    }

    count == 4
}

fn packet_rows(
    packets: &[PacketSummary],
) -> Vec<(
    String,
    Option<String>,
    String,
    String,
    String,
    usize,
    String,
)> {
    packets
        .iter()
        .map(|packet| {
            (
                packet.timestamp.clone(),
                packet.interface.clone(),
                packet.source.clone(),
                packet.destination.clone(),
                packet.protocol.clone(),
                packet.length,
                packet.summary.clone(),
            )
        })
        .collect()
}

fn select_filter_dimension(app: &mut App, target: FilterDimension) {
    for _ in 0..app.filter_dimensions().len() {
        if app.selected_filter_dimension() == target {
            return;
        }
        app.next_filter_dimension();
    }

    panic!("failed to select filter dimension: {target:?}");
}

fn read_packets_via_api(pcap_path: &Path) -> Result<Vec<PacketSummary>, Box<dyn Error>> {
    let api = TcpdumpApi::default();
    let packets = api.read_pcap(TcpdumpReadRequest {
        pcap_path,
        filter_args: &[],
    })?;
    Ok(packets)
}

fn run_tcpdump_packets(
    pcap_path: &Path,
    filter_args: &[String],
) -> Result<Vec<PacketSummary>, Box<dyn Error>> {
    let output = Command::new("tcpdump")
        .arg("-nn")
        .arg("-tttt")
        .arg("-r")
        .arg(pcap_path)
        .args(filter_args)
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "tcpdump failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let stdout = String::from_utf8(output.stdout)?;
    Ok(parse_tcpdump_stdout(&stdout))
}

fn fixture_path(label: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!(
        "lazytcp_{}_{}_{}.pcap",
        label,
        std::process::id(),
        timestamp
    ))
}

fn write_filter_parity_fixture(path: &Path) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(path)?;

    // pcap global header (little-endian), LINKTYPE_RAW (101) so frames start with IP bytes.
    file.write_all(&0xa1b2c3d4_u32.to_le_bytes())?;
    file.write_all(&2_u16.to_le_bytes())?;
    file.write_all(&4_u16.to_le_bytes())?;
    file.write_all(&0_i32.to_le_bytes())?;
    file.write_all(&0_u32.to_le_bytes())?;
    file.write_all(&65535_u32.to_le_bytes())?;
    file.write_all(&101_u32.to_le_bytes())?;

    let mut timestamp_sec = 1_u32;
    let mut packet_id = 1_u16;

    let tcp_flags = [
        (true, 0x00_u8),  // NS
        (false, 0x80_u8), // CWR
        (false, 0x40_u8), // ECE
        (false, 0x20_u8), // URG
        (false, 0x10_u8), // ACK
        (false, 0x08_u8), // PSH
        (false, 0x04_u8), // RST
        (false, 0x02_u8), // SYN
        (false, 0x01_u8), // FIN
    ];

    for (index, (ns, flags)) in tcp_flags.iter().enumerate() {
        let packet = build_ipv4_tcp_packet(
            [10, 0, 0, 10],
            [1, 1, 1, 1],
            40_000 + index as u16,
            443,
            *ns,
            *flags,
            packet_id,
        );
        write_packet_record(&mut file, timestamp_sec, timestamp_sec * 1_000, &packet)?;
        timestamp_sec += 1;
        packet_id += 1;
    }

    let udp_unicast = build_ipv4_udp_packet([10, 0, 0, 20], [8, 8, 8, 8], 53_000, 53, packet_id);
    write_packet_record(
        &mut file,
        timestamp_sec,
        timestamp_sec * 1_000,
        &udp_unicast,
    )?;
    timestamp_sec += 1;
    packet_id += 1;

    let udp_multicast =
        build_ipv4_udp_packet([10, 0, 0, 21], [224, 0, 0, 251], 5_353, 5_353, packet_id);
    write_packet_record(
        &mut file,
        timestamp_sec,
        timestamp_sec * 1_000,
        &udp_multicast,
    )?;
    timestamp_sec += 1;
    packet_id += 1;

    let udp_broadcast = build_ipv4_udp_packet([0, 0, 0, 0], [192, 168, 0, 255], 68, 67, packet_id);
    write_packet_record(
        &mut file,
        timestamp_sec,
        timestamp_sec * 1_000,
        &udp_broadcast,
    )?;
    timestamp_sec += 1;
    packet_id += 1;

    let icmp = build_ipv4_icmp_packet([10, 0, 0, 12], [1, 1, 1, 1], packet_id);
    write_packet_record(&mut file, timestamp_sec, timestamp_sec * 1_000, &icmp)?;
    timestamp_sec += 1;

    let udp_ipv6 = build_ipv6_udp_packet(
        [
            0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0x18, 0xeb, 0x9e, 0x78, 0xfb, 0x61, 0x60, 0x6f,
        ],
        [0xff, 0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xfb],
        5_353,
        5_353,
    );
    write_packet_record(&mut file, timestamp_sec, timestamp_sec * 1_000, &udp_ipv6)?;
    timestamp_sec += 1;

    let icmp6 = build_ipv6_icmp6_packet(
        [
            0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0xc4, 0x7, 0x45, 0x92, 0xb9, 0x9c, 0x38, 0x23,
        ],
        [0xff, 0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x01],
    );
    write_packet_record(&mut file, timestamp_sec, timestamp_sec * 1_000, &icmp6)?;

    Ok(())
}

fn build_ipv4_tcp_packet(
    source: [u8; 4],
    destination: [u8; 4],
    source_port: u16,
    destination_port: u16,
    ns: bool,
    flags: u8,
    identification: u16,
) -> Vec<u8> {
    let mut packet = build_ipv4_header(6, source, destination, 20, identification);

    packet.extend_from_slice(&source_port.to_be_bytes());
    packet.extend_from_slice(&destination_port.to_be_bytes());
    packet.extend_from_slice(&1_u32.to_be_bytes());
    packet.extend_from_slice(&0_u32.to_be_bytes());
    packet.push(0x50 | u8::from(ns));
    packet.push(flags);
    packet.extend_from_slice(&0x4000_u16.to_be_bytes());
    packet.extend_from_slice(&0_u16.to_be_bytes());
    packet.extend_from_slice(&0_u16.to_be_bytes());

    packet
}

fn build_ipv4_udp_packet(
    source: [u8; 4],
    destination: [u8; 4],
    source_port: u16,
    destination_port: u16,
    identification: u16,
) -> Vec<u8> {
    let mut packet = build_ipv4_header(17, source, destination, 8, identification);

    packet.extend_from_slice(&source_port.to_be_bytes());
    packet.extend_from_slice(&destination_port.to_be_bytes());
    packet.extend_from_slice(&8_u16.to_be_bytes());
    packet.extend_from_slice(&0_u16.to_be_bytes());

    packet
}

fn build_ipv4_icmp_packet(source: [u8; 4], destination: [u8; 4], identification: u16) -> Vec<u8> {
    let mut packet = build_ipv4_header(1, source, destination, 8, identification);

    packet.extend_from_slice(&[8, 0, 0, 0, 0, 1, 0, 1]);

    packet
}

fn build_ipv6_udp_packet(
    source: [u8; 16],
    destination: [u8; 16],
    source_port: u16,
    destination_port: u16,
) -> Vec<u8> {
    let mut packet = build_ipv6_header(17, source, destination, 8);

    packet.extend_from_slice(&source_port.to_be_bytes());
    packet.extend_from_slice(&destination_port.to_be_bytes());
    packet.extend_from_slice(&8_u16.to_be_bytes());
    packet.extend_from_slice(&0_u16.to_be_bytes());

    packet
}

fn build_ipv6_icmp6_packet(source: [u8; 16], destination: [u8; 16]) -> Vec<u8> {
    let mut packet = build_ipv6_header(58, source, destination, 8);

    packet.extend_from_slice(&[135, 0, 0, 0, 0, 0, 0, 0]);

    packet
}

fn build_ipv4_header(
    protocol: u8,
    source: [u8; 4],
    destination: [u8; 4],
    payload_length: usize,
    identification: u16,
) -> Vec<u8> {
    let total_length = (20 + payload_length) as u16;
    let mut header = Vec::with_capacity(20);

    header.push(0x45);
    header.push(0x00);
    header.extend_from_slice(&total_length.to_be_bytes());
    header.extend_from_slice(&identification.to_be_bytes());
    header.extend_from_slice(&0x4000_u16.to_be_bytes());
    header.push(64);
    header.push(protocol);
    header.extend_from_slice(&0_u16.to_be_bytes());
    header.extend_from_slice(&source);
    header.extend_from_slice(&destination);

    header
}

fn build_ipv6_header(
    next_header: u8,
    source: [u8; 16],
    destination: [u8; 16],
    payload_length: usize,
) -> Vec<u8> {
    let mut header = Vec::with_capacity(40);

    header.extend_from_slice(&[0x60, 0x00, 0x00, 0x00]);
    header.extend_from_slice(&(payload_length as u16).to_be_bytes());
    header.push(next_header);
    header.push(64);
    header.extend_from_slice(&source);
    header.extend_from_slice(&destination);

    header
}

fn write_packet_record(
    writer: &mut File,
    ts_sec: u32,
    ts_usec: u32,
    packet: &[u8],
) -> Result<(), Box<dyn Error>> {
    let packet_len = packet.len() as u32;
    writer.write_all(&ts_sec.to_le_bytes())?;
    writer.write_all(&ts_usec.to_le_bytes())?;
    writer.write_all(&packet_len.to_le_bytes())?;
    writer.write_all(&packet_len.to_le_bytes())?;
    writer.write_all(packet)?;

    Ok(())
}

fn tcpdump_available() -> bool {
    Command::new("tcpdump").arg("--version").output().is_ok()
}
