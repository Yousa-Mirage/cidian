use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use cidian::{Dictionary, Result, bdict, qcel, qpyd, scel};

const ITERATIONS: usize = 10;

type Parser = fn(&[u8]) -> Result<Dictionary>;

#[derive(Clone, Copy)]
struct Case {
    name: &'static str,
    directory: &'static str,
    file: &'static str,
    parser: Parser,
}

const CASES: [Case; 4] = [
    Case {
        name: "SCEL",
        directory: "scel",
        file: "医学词汇大全.scel",
        parser: scel::parse,
    },
    Case {
        name: "QCEL",
        directory: "qcel",
        file: "成语俗语大全.qcel",
        parser: qcel::parse,
    },
    Case {
        name: "QPYD",
        directory: "qpyd",
        file: "唐诗.qpyd",
        parser: qpyd::parse,
    },
    Case {
        name: "BDICT",
        directory: "baidu",
        file: "诗词精选.bdict",
        parser: bdict::parse,
    },
];

fn main() {
    println!("cidian-rs parser benchmark: {ITERATIONS} parses per fixture");

    for case in CASES {
        benchmark_case(case);
    }
}

fn benchmark_case(case: Case) {
    let path = fixture_path(case);
    let data = match fs::read(&path) {
        Ok(data) => data,
        Err(error) => panic!(
            "failed to read benchmark fixture {}: {error}",
            path.display()
        ),
    };

    // Warm up the parser once so one-time initialization does not affect the
    // reported measurements.
    let warmup = parse(case, &data, &path);
    black_box(warmup);

    let mut durations = Vec::with_capacity(ITERATIONS);
    let mut entry_count = 0;
    for _ in 0..ITERATIONS {
        let start = Instant::now();
        let dictionary = parse(case, &data, &path);
        let elapsed = start.elapsed();
        entry_count = dictionary.entries.len();
        black_box(dictionary);
        durations.push(elapsed);
    }

    durations.sort_unstable();
    let minimum = durations[0];
    let average_ms = durations
        .iter()
        .map(|duration| duration.as_secs_f64())
        .sum::<f64>()
        * 1_000.0
        / durations.len() as f64;

    println!(
        "format={} file={} bytes={} entries={} min_ms={:.2} avg_ms={:.2}",
        case.name,
        case.file,
        data.len(),
        entry_count,
        milliseconds(minimum),
        average_ms,
    );
}

fn fixture_path(case: Case) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(case.directory)
        .join(case.file)
}

fn parse(case: Case, data: &[u8], path: &Path) -> Dictionary {
    match (case.parser)(black_box(data)) {
        Ok(dictionary) => dictionary,
        Err(error) => panic!(
            "failed to parse benchmark fixture {}: {error}",
            path.display()
        ),
    }
}

fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}
