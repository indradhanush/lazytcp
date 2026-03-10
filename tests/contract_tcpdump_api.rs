use std::collections::BTreeSet;
use std::error::Error;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use lazytcp::api::{TcpdumpApi, TcpdumpReadRequest};
use lazytcp::app::App;

#[test]
fn api_matches_tcpdump_contract_without_filter() -> Result<(), Box<dyn Error>> {
    if !tcpdump_available() {
        eprintln!("skipping contract test: tcpdump is not installed");
        return Ok(());
    }

    let pcap_path = fixture_path("contract-all");
    write_fixture_pcap(&pcap_path)?;

    let api = TcpdumpApi::default();
    let packets = api.read_pcap(TcpdumpReadRequest {
        pcap_path: &pcap_path,
        filter_args: &[],
    })?;

    let direct_stdout = run_tcpdump(&pcap_path, &[])?;
    let baseline = baseline_contract_rows(&direct_stdout);
    let api_rows: Vec<(String, String, usize)> = packets
        .iter()
        .map(|packet| {
            (
                packet.source.clone(),
                packet.destination.clone(),
                packet.length,
            )
        })
        .collect();

    assert_eq!(api_rows, baseline);

    fs::remove_file(&pcap_path)?;
    Ok(())
}

#[test]
fn api_matches_tcpdump_contract_with_udp_filter() -> Result<(), Box<dyn Error>> {
    if !tcpdump_available() {
        eprintln!("skipping contract test: tcpdump is not installed");
        return Ok(());
    }

    let pcap_path = fixture_path("contract-udp");
    write_fixture_pcap(&pcap_path)?;

    let api = TcpdumpApi::default();
    let packets = api.read_pcap(TcpdumpReadRequest {
        pcap_path: &pcap_path,
        filter_args: &["udp"],
    })?;

    let direct_stdout = run_tcpdump(&pcap_path, &["udp"])?;
    let baseline = baseline_contract_rows(&direct_stdout);
    let api_rows: Vec<(String, String, usize)> = packets
        .iter()
        .map(|packet| {
            (
                packet.source.clone(),
                packet.destination.clone(),
                packet.length,
            )
        })
        .collect();

    assert_eq!(api_rows, baseline);
    assert_eq!(packets.len(), 1);
    assert_eq!(packets[0].protocol, "UDP");

    fs::remove_file(&pcap_path)?;
    Ok(())
}

#[test]
fn host_filter_candidates_match_tcpdump_contract() -> Result<(), Box<dyn Error>> {
    if !tcpdump_available() {
        eprintln!("skipping contract test: tcpdump is not installed");
        return Ok(());
    }

    let pcap_path = fixture_path("contract-hosts");
    write_fixture_pcap(&pcap_path)?;

    let api = TcpdumpApi::default();
    let packets = api.read_pcap(TcpdumpReadRequest {
        pcap_path: &pcap_path,
        filter_args: &[],
    })?;

    let mut app = App::with_packets(packets, String::new());
    app.open_filter_popup();
    let actual_hosts = app
        .filter_popup_candidates()
        .map(|candidates| candidates.to_vec())
        .unwrap_or_default();

    let direct_stdout = run_tcpdump(&pcap_path, &[])?;
    let expected_hosts = tcpdump_host_candidates(&direct_stdout);

    assert_eq!(actual_hosts, expected_hosts);

    fs::remove_file(&pcap_path)?;
    Ok(())
}

fn run_tcpdump(path: &Path, filter_args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = Command::new("tcpdump")
        .arg("-nn")
        .arg("-tttt")
        .arg("-r")
        .arg(path)
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

    Ok(String::from_utf8(output.stdout)?)
}

fn baseline_contract_rows(stdout: &str) -> Vec<(String, String, usize)> {
    stdout
        .lines()
        .filter_map(|line| {
            let (_, payload) = line
                .split_once(" IP ")
                .or_else(|| line.split_once(" IP6 "))?;

            let (path_segment, summary) = payload.split_once(": ").unwrap_or((payload, ""));
            let (source, destination) = path_segment.split_once(" > ")?;
            let length = parse_length(summary).unwrap_or(0);

            Some((
                source.trim().to_string(),
                destination.trim().trim_end_matches(':').to_string(),
                length,
            ))
        })
        .collect()
}

fn tcpdump_host_candidates(stdout: &str) -> Vec<String> {
    let mut hosts = BTreeSet::new();

    for line in stdout.lines() {
        let Some((_, payload)) = line.split_once(" IP ").or_else(|| line.split_once(" IP6 "))
        else {
            continue;
        };

        let (path_segment, _) = payload.split_once(": ").unwrap_or((payload, ""));
        let Some((source, destination)) = path_segment.split_once(" > ") else {
            continue;
        };

        hosts.insert(normalize_host_endpoint(source.trim()));
        hosts.insert(normalize_host_endpoint(
            destination.trim().trim_end_matches(':'),
        ));
    }

    hosts.into_iter().collect()
}

fn normalize_host_endpoint(endpoint: &str) -> String {
    if let Some((host, port)) = endpoint.rsplit_once('.') {
        if !host.is_empty()
            && !port.is_empty()
            && port.chars().all(|ch| ch.is_ascii_digit())
            && (host.contains(':') || is_ipv4_address(host))
        {
            return host.to_ascii_lowercase();
        }
    }

    endpoint.to_ascii_lowercase()
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

fn parse_length(summary: &str) -> Option<usize> {
    let index = summary.rfind("length ")?;
    let remainder = &summary[index + "length ".len()..];

    let digits: String = remainder
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect();

    if digits.is_empty() {
        return None;
    }

    digits.parse::<usize>().ok()
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

fn write_fixture_pcap(path: &Path) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(path)?;

    // pcap global header (little-endian), LINKTYPE_RAW (101) so frames start with IPv4 bytes.
    file.write_all(&0xa1b2c3d4_u32.to_le_bytes())?;
    file.write_all(&2_u16.to_le_bytes())?;
    file.write_all(&4_u16.to_le_bytes())?;
    file.write_all(&0_i32.to_le_bytes())?;
    file.write_all(&0_u32.to_le_bytes())?;
    file.write_all(&65535_u32.to_le_bytes())?;
    file.write_all(&101_u32.to_le_bytes())?;

    let tcp_packet: [u8; 40] = [
        0x45, 0x00, 0x00, 0x28, 0x00, 0x01, 0x40, 0x00, 0x40, 0x06, 0x00, 0x00, 0x0A, 0x00, 0x00,
        0x0C, 0x01, 0x01, 0x01, 0x01, 0xC9, 0x58, 0x01, 0xBB, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
        0x00, 0x00, 0x50, 0x02, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00,
    ];

    let udp_packet: [u8; 28] = [
        0x45, 0x00, 0x00, 0x1C, 0x00, 0x02, 0x40, 0x00, 0x40, 0x11, 0x00, 0x00, 0x0A, 0x00, 0x00,
        0x0C, 0x08, 0x08, 0x08, 0x08, 0x85, 0xA3, 0x00, 0x35, 0x00, 0x08, 0x00, 0x00,
    ];

    let icmp_packet: [u8; 28] = [
        0x45, 0x00, 0x00, 0x1C, 0x00, 0x03, 0x40, 0x00, 0x40, 0x01, 0x00, 0x00, 0x0A, 0x00, 0x00,
        0x0C, 0x01, 0x01, 0x01, 0x01, 0x08, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01,
    ];

    write_packet_record(&mut file, 1, 1_000, &tcp_packet)?;
    write_packet_record(&mut file, 2, 2_000, &udp_packet)?;
    write_packet_record(&mut file, 3, 3_000, &icmp_packet)?;

    Ok(())
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
