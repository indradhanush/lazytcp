#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_dir="${CARGO_TARGET_DIR:-"$repo_root/target"}"
coverage_dir="$target_dir/coverage"
raw_dir="$coverage_dir/raw"
profdata_file="$coverage_dir/lazytcp.profdata"
lcov_file="$coverage_dir/lcov.info"
html_dir="$coverage_dir/html"
summary_file="$coverage_dir/summary.txt"
artifacts_file="$coverage_dir/test-artifacts.jsonl"
ignore_filename_regex='/.cargo/registry|/rustc/|/toolchains/.*/lib/rustlib/src/rust/library/'

host_triple="$(rustc -vV | sed -n 's/^host: //p')"
sysroot="$(rustc --print sysroot)"
llvm_tools_dir="$sysroot/lib/rustlib/$host_triple/bin"
llvm_profdata="$llvm_tools_dir/llvm-profdata"
llvm_cov="$llvm_tools_dir/llvm-cov"

if [[ ! -x "$llvm_profdata" || ! -x "$llvm_cov" ]]; then
  cat >&2 <<EOF
error: rustup llvm-tools component is required for coverage reporting.
install with:
  rustup component add llvm-tools
EOF
  exit 1
fi

rm -rf "$coverage_dir"
mkdir -p "$raw_dir"

echo "running tests with source-based coverage instrumentation..."
(
  cd "$repo_root"
  RUSTFLAGS="-C instrument-coverage" \
  CARGO_INCREMENTAL=0 \
  LLVM_PROFILE_FILE="$raw_dir/build-%p-%m.profraw" \
  cargo test --tests --lib --bins --no-run --message-format=json >"$artifacts_file"

  RUSTFLAGS="-C instrument-coverage" \
  CARGO_INCREMENTAL=0 \
  LLVM_PROFILE_FILE="$raw_dir/run-%p-%m.profraw" \
  cargo test
)

shopt -s nullglob
profraw_files=("$raw_dir"/*.profraw)
if [[ ${#profraw_files[@]} -eq 0 ]]; then
  echo "error: no profraw files were generated in $raw_dir" >&2
  exit 1
fi

echo "merging coverage profiles..."
"$llvm_profdata" merge -sparse "${profraw_files[@]}" -o "$profdata_file"

objects=()
lib_objects=()
other_objects=()

while IFS=$'\t' read -r object_kind candidate; do
  candidate="${candidate//\\\//\/}"
  [[ -f "$candidate" ]] || continue
  [[ -x "$candidate" ]] || continue

  if [[ "$object_kind" == "lib" ]]; then
    lib_objects+=("$candidate")
  else
    other_objects+=("$candidate")
  fi
done < <(
  awk '
    /"profile":\{[^}]*"test":true/ && /"executable":"/ {
      kind = "other"
      if ($0 ~ /"target":\{"kind":\["lib"\]/) {
        kind = "lib"
      }
      path = $0
      sub(/^.*"executable":"/, "", path)
      sub(/".*$/, "", path)
      print kind "\t" path
    }
  ' "$artifacts_file"
)

add_unique_object() {
  local candidate="$1"
  local existing
  for existing in "${objects[@]}"; do
    [[ "$existing" == "$candidate" ]] && return
  done
  objects+=("$candidate")
}

for candidate in "${lib_objects[@]}" "${other_objects[@]}"; do
  add_unique_object "$candidate"
done

if [[ ${#objects[@]} -eq 0 ]]; then
  echo "error: no instrumented test binaries found in $artifacts_file" >&2
  exit 1
fi

echo "writing terminal summary to $summary_file..."
"$llvm_cov" report \
  --instr-profile="$profdata_file" \
  --ignore-filename-regex="$ignore_filename_regex" \
  "${objects[@]}" | tee "$summary_file"

echo "exporting lcov report to $lcov_file..."
"$llvm_cov" export \
  --format=lcov \
  --instr-profile="$profdata_file" \
  --ignore-filename-regex="$ignore_filename_regex" \
  "${objects[@]}" > "$lcov_file"

echo "rendering html report to $html_dir..."
mkdir -p "$html_dir"
"$llvm_cov" show \
  --format=html \
  --output-dir="$html_dir" \
  --instr-profile="$profdata_file" \
  --ignore-filename-regex="$ignore_filename_regex" \
  "${objects[@]}" >/dev/null

echo "coverage artifacts:"
echo "  summary: $summary_file"
echo "  lcov:    $lcov_file"
echo "  html:    $html_dir/index.html"
