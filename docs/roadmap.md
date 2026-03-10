# lazytcp roadmap

## Prioritized next builds

1. `tcpdump` parity matrix for all filters.
   - Add contract tests for each filter category against `tcpdump` output on fixture pcaps.
   - Include Date Time range, TCP Flags, Source Port, and Destination Port.
2. Live capture mode.
   - Stream packets from a selected interface while preserving current interactive filtering workflow.
3. Flow-aware view.
   - Group packets by 5-tuple style conversation and show packet/byte counters with drill-down.
4. Date Time filter UX hardening.
   - Inline parse validation, clearer error states, relative time input (for example `-5m`), and local/UTC toggle.
5. Filter presets.
   - Save/apply named filter sets for common investigations.
6. Popup candidate search.
   - Type-to-filter for large host/port/value lists in selection popups.
7. Deeper packet detail decoding.
   - Expand protocol-aware detail rendering (for example DNS/TLS/HTTP basics when present).
8. Session export.
   - Export filtered packets to a new `.pcap` and optionally export current filter expression.

## Suggested execution order

- Start with parity tests first to lock correctness before additional UI/backend complexity.
- Land live capture mode next to meet the primary workflow goal.
- Then iterate on flow view and UX enhancements with parity suite as a regression gate.

## Scope guardrail

- Keep the product focused on rapid terminal-based packet exploration.
- Avoid broad Wireshark-style feature expansion unless explicitly requested.
