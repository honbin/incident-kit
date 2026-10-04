# incident-kit

Small Unix-style tools for incident investigation, written in Rust.

Each tool reads from stdin, does one thing, and writes to stdout — so they
compose with `grep` / `jq` / `awk` and with each other.

## Tools

| tool                 | what it does                                       |
|----------------------|----------------------------------------------------|
| `incident-burst`     | Detect bursts in timestamped events                |
| `incident-window`    | Pass through lines whose timestamp is in a range   |
| `incident-dist`      | Summarize the distribution of a column of numbers  |
| `incident-correlate` | How often two timestamped streams co-occur in time |
| `incident-series`    | View a `timestamp value` series as a sparkline     |
| `incident-align`     | Inner-join timestamped series on their timestamp   |
| `incident-timeline`  | Stack timestamp/value series on a shared time axis |

The binaries are namespaced (`incident-*`) so they don't clash with other tools
on your `PATH`; alias them to shorter names if you like.

## Install

Install the `incident-*` binaries straight from GitHub:

```
cargo install --git https://github.com/honbin/incident-kit --locked \
  burst window dist correlate series align timeline
```

Cargo installs them to its default bin directory (`~/.cargo/bin`) — make sure
that's on your `PATH`.

Install only the tools you want — just name fewer:

```
cargo install --git https://github.com/honbin/incident-kit --locked burst
```

Developing from a local clone? Use `--path` instead:

```
cargo install --locked --path crates/burst
```

## Timestamps

Every tool expects RFC3339 timestamps with an explicit offset, such as
`2026-10-03T10:00:00+09:00` or `…Z`. A shared timestamp contract keeps the
tools composable and avoids guessing:

- **unit** — bare epoch values don't say whether they are seconds,
  milliseconds, or microseconds.
- **display offset** — `incident-burst`, `incident-series`, and
  `incident-timeline` render times in the input's offset, which an epoch value
  doesn't carry.

Other formats should be normalized to RFC3339 before piping them into
incident-kit — a job for the shell. The examples use `gawk` (macOS: `brew
install gawk`):

```sh
# epoch seconds -> RFC3339 UTC ($1/1000 for ms)
gawk '{ print strftime("%Y-%m-%dT%H:%M:%SZ", $1, 1) }'

# Apache/CLF  [03/Oct/2026:10:21:31 +0900]  ->  2026-10-03T10:21:31+09:00
gawk 'BEGIN { split("Jan Feb Mar Apr May Jun Jul Aug Sep Oct Nov Dec", a, " ")
              for (i = 1; i <= 12; i++) m[a[i]] = sprintf("%02d", i) }
      match($0, /([0-9]{2})\/([A-Za-z]{3})\/([0-9]{4}):([0-9:]{8}) ([+-][0-9]{2})([0-9]{2})/, x) {
          print x[3]"-"m[x[2]]"-"x[1]"T"x[4] x[5]":"x[6] }'
```

## Layout

A Cargo workspace; one crate per tool under `crates/`, plus `tstamp`, a small
shared crate holding just the RFC3339 timestamp parser. It was extracted once a
third tool needed the identical parser (the rule of three); it stays narrow on
purpose — durations, histograms, and formatting do not belong in it. Tools still
duplicate trivial helpers until duplication is proven.

## Build & test

```
cargo build
cargo test
```

Three tiers, each narrower and cheaper than the last:

- **unit** (`src/lib.rs` per crate) — pure logic: parse / bucket / summarize.
- **per-tool CLI** (`crates/<tool>/tests/cli.rs`) — one binary: stdin → stdout →
  exit code.
- **integration** (`crates/integration-tests/`) — the tools *composed*: real
  pipelines like `incident-window | incident-burst`, kept to a few representative
  flows plus a golden fixture (`fixtures/alb-502-incident/`) that must still
  rediscover a known peak. Run via a whole-workspace `cargo test` so every binary
  is built.

## incident-burst

```
# ALB 502s per second
jq -r 'select(.status == 502) | .timestamp' alb.jsonl | incident-burst

# SIGTERM occurrences
grep SIGTERM app.log | awk '{print $1}' | incident-burst
```

Input: one RFC3339 timestamp per line, offset required (e.g.
`2026-09-03T10:21:31.124+09:00` or `…Z`). All lines must share the same offset —
mixed offsets are rejected.
Output: a per-second histogram, then total / peak rate / first / last / span.
Times are shown in the input's offset so they line up with the source log and
CloudWatch without conversion.

## incident-window

```
# narrow a raw log to the incident window, then look at the shape
cat alb.jsonl \
  | incident-window --from 2026-09-03T10:20:00+09:00 --to 2026-09-03T10:30:00+09:00 \
  | jq -r 'select(.elb_status_code == 502) | .time' \
  | incident-burst
```

Reads the RFC3339 timestamp from each line (leading token, quotes stripped) and
prints the original line unchanged when it falls in `[--from, --to]` (inclusive;
at least one bound required). Comparison is on the instant, so the bounds and the
lines may use any offset. A `kept N / total` summary goes to stderr.

## incident-dist

```
# response-time distribution for the incident window
cat alb.jsonl \
  | incident-window --from 2026-09-03T10:20:00+09:00 --to 2026-09-03T10:30:00+09:00 \
  | jq -r .target_processing_time \
  | incident-dist
```

Reads one number per line (leading token, quotes stripped; NaN/inf rejected) and
prints count / min / mean / p50 / p90 / p95 / p99 / max / stddev. Unit-agnostic —
feed ms, seconds, or counts and read the result in the same unit. Percentiles are
nearest-rank (no interpolation), so each is an actually-observed value.

## incident-correlate

```
# did the 502s and the SIGTERMs cluster at the same time?
incident-correlate 502.txt sigterm.txt --window 5s
```

Reads one RFC3339 timestamp per line from two files and reports, in both
directions, what fraction of one stream's events have an event in the other
within the specified ±window (`5s` / `2m` / `1h` / bare seconds; default 1s).
Both directions matter — "71% of 502s were near a SIGTERM" and "83% of SIGTERMs
were near a 502"
are different facts. It counts matched events, not pairs.

## incident-series

```
# shape of a CloudWatch metric over the incident window, with the peak minute
aws cloudwatch get-metric-statistics ... \
  --query 'Datapoints[].[Timestamp,Sum]' --output text \
  | incident-series
```

Reads `RFC3339 <whitespace> number` lines (order doesn't matter — sorted
internally), one point per line. Prints a row per point (time, sparkline level,
value), then points / sum / min / max with the time of each extreme. The
sparkline is a linear map onto `▁▂▃▄▅▆▇█`. Unlike `incident-burst` (which counts
raw event occurrences), `incident-series` consumes values that are already
aggregated per timestamp — e.g. CloudWatch datapoints.

## incident-align

Inner-join N `timestamp value` files on their timestamp. For every timestamp
present in *all* inputs it prints `<RFC3339>` then one tab-separated value per
file, sorted by time. Only the join lives in `incident-align` — the fragile part
to do in shell (nested `join`); the derived metric and its peak are a downstream
step, which keeps the output re-parseable by `incident-series`:

```
# per-task concurrency ≈ (rate × latency) / tasks, and when it peaked
incident-align req.txt lat.txt tasks.txt \
  | awk -F'\t' '{printf "%s\t%.2f\n", $1, $2/60*$3/$4}' \
  | incident-series
```

where req / lat / tasks are CloudWatch RequestCount (Sum), TargetResponseTime
(Average) and LiveTaskCount (Minimum). A `common N / per-file totals` line goes to
stderr so dropped (non-common) timestamps are visible.

## incident-timeline

```
# a 502 burst next to CPU / latency / task count on one time axis
incident-timeline \
  --series 502.txt --series cpu.txt --series latency.txt --series tasks.txt \
  > report.html
```

Reads one or more `RFC3339 <whitespace> number` files via `--series` (same format
as `incident-series`; a per-second log count from `stats count() by bin(1s)` drops
in exactly like a metric). Writes a self-contained HTML page — inline SVG, no
JavaScript, no external assets — to stdout, ready to open or share.

Each series becomes one line chart with a marker per point, on its own y-scale,
stacked on a shared time axis, so you can see what moved together and when. The
line only connects the points you supply — it doesn't imply a value in the gaps,
so zero-fill upstream if you need that. Markers carry the full timestamp and value
as a hover tooltip; each chart labels its own min / max.

Unlike the summary tools, this one is for *shape and simultaneity* across signals —
the part a per-metric summary drops. Like `incident-correlate` and `incident-align`
it reads files (not stdin), and it emits HTML rather than text.

## License

[MIT](LICENSE)
