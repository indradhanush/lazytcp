use std::path::Path;
use std::process::Command;

use crate::capture::CaptureError;
use crate::domain::PacketSummary;

#[derive(Debug, Clone, Copy)]
pub struct TcpdumpReadRequest<'a> {
    pub pcap_path: &'a Path,
    pub filter_args: &'a [&'a str],
}

pub struct TcpdumpApi {
    binary: String,
}

impl TcpdumpApi {
    pub fn new(binary: impl Into<String>) -> Self {
        Self {
            binary: binary.into(),
        }
    }

    pub fn read_pcap(
        &self,
        request: TcpdumpReadRequest<'_>,
    ) -> Result<Vec<PacketSummary>, CaptureError> {
        let mut command = Command::new(&self.binary);
        command
            .arg("-nn")
            .arg("-tttt")
            .arg("-r")
            .arg(request.pcap_path)
            .args(request.filter_args);

        let output = command.output().map_err(|err| {
            CaptureError::new(format!("failed to execute tcpdump command: {err}"))
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CaptureError::new(format!(
                "tcpdump exited with status {}: {}",
                output.status,
                stderr.trim()
            )));
        }

        let stdout = String::from_utf8(output.stdout).map_err(|err| {
            CaptureError::new(format!("tcpdump output was not valid UTF-8: {err}"))
        })?;

        Ok(parse_tcpdump_stdout(&stdout))
    }
}

impl Default for TcpdumpApi {
    fn default() -> Self {
        Self::new("tcpdump")
    }
}

pub fn parse_tcpdump_stdout(stdout: &str) -> Vec<PacketSummary> {
    stdout.lines().filter_map(parse_tcpdump_line).collect()
}

pub fn parse_tcpdump_line(line: &str) -> Option<PacketSummary> {
    let (timestamp, payload) = line
        .split_once(" IP ")
        .or_else(|| line.split_once(" IP6 "))?;

    let (path_segment, summary) = payload.split_once(": ").unwrap_or((payload, ""));
    let (source, destination) = path_segment.split_once(" > ")?;

    Some(PacketSummary {
        timestamp: timestamp.trim().to_string(),
        source: source.trim().to_string(),
        destination: destination.trim().trim_end_matches(':').to_string(),
        protocol: classify_protocol(source, destination, summary),
        length: parse_length(summary).unwrap_or(0),
        summary: summary.trim().to_string(),
    })
}

fn classify_protocol(source: &str, destination: &str, summary: &str) -> String {
    if summary.contains("Flags [") {
        "TCP".to_string()
    } else if summary.contains("UDP") {
        "UDP".to_string()
    } else if summary.contains("ICMP6") {
        "ICMP6".to_string()
    } else if summary.contains("ICMP") {
        "ICMP".to_string()
    } else if endpoint_has_numeric_port(source) && endpoint_has_numeric_port(destination) {
        // tcpdump's terse UDP output can omit the word "UDP" entirely.
        "UDP".to_string()
    } else {
        "IP".to_string()
    }
}

fn endpoint_has_numeric_port(endpoint: &str) -> bool {
    endpoint
        .rsplit_once('.')
        .is_some_and(|(_, port)| !port.is_empty() && port.chars().all(|ch| ch.is_ascii_digit()))
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

#[cfg(test)]
mod tests {
    use super::parse_tcpdump_line;

    #[test]
    fn parses_tcp_line_into_packet_summary() {
        let line = "1970-01-01 00:00:01.001000 IP 10.0.0.12.51544 > 1.1.1.1.443: Flags [S], seq 1, win 65535, length 0";
        let packet = parse_tcpdump_line(line).expect("line should parse");

        assert_eq!(packet.protocol, "TCP");
        assert_eq!(packet.source, "10.0.0.12.51544");
        assert_eq!(packet.destination, "1.1.1.1.443");
        assert_eq!(packet.length, 0);
    }

    #[test]
    fn parses_udp_line_into_packet_summary() {
        let line = "1970-01-01 00:00:02.002000 IP 10.0.0.12.34211 > 8.8.8.8.53: UDP, length 0";
        let packet = parse_tcpdump_line(line).expect("line should parse");

        assert_eq!(packet.protocol, "UDP");
        assert_eq!(packet.source, "10.0.0.12.34211");
        assert_eq!(packet.destination, "8.8.8.8.53");
        assert_eq!(packet.length, 0);
    }
}
