// Decode/roundtrip benchmark for the production libjxl `djxl` decode path
// (codec::decode_jxl). Also exercises concurrent decodes, since production
// fires many per action. Gated with #[ignore] — needs ./jxl_from_tree + ./djxl
// and should be run in RELEASE:
//
//     cargo test --release --test decode_bench -- --ignored --nocapture
//
// from the project root.

use std::time::{Duration, Instant};

use artxl::{codec, render};

const K: usize = 5; // timed iterations per case

fn prog(w: u32, h: u32, extra: &str) -> String {
    let mut s = format!("Bitdepth 8\nWidth {w}\nHeight {h}\n");
    if !extra.is_empty() {
        s.push_str(extra);
        s.push('\n');
    }
    s.push_str("\nif c > 0\n  if W+N-NW > 80\n    - W + 4\n    - AvgW+NW + 6\n  - Gradient + 3\n");
    s
}

fn mean_min(times: &[Duration]) -> (f64, f64) {
    let ms: Vec<f64> = times.iter().map(|d| d.as_secs_f64() * 1000.0).collect();
    let mean = ms.iter().sum::<f64>() / ms.len() as f64;
    let min = ms.iter().cloned().fold(f64::INFINITY, f64::min);
    (mean, min)
}

fn mean_rgb(rgba: &[u8]) -> (f64, f64, f64) {
    let (mut r, mut g, mut b) = (0u64, 0u64, 0u64);
    let n = (rgba.len() / 4).max(1) as u64;
    for px in rgba.chunks_exact(4) {
        r += px[0] as u64;
        g += px[1] as u64;
        b += px[2] as u64;
    }
    (r as f64 / n as f64, g as f64 / n as f64, b as f64 / n as f64)
}

#[test]
#[ignore]
fn decode_roundtrip_bench() {
    if !std::path::Path::new("./jxl_from_tree").exists() || !std::path::Path::new("./djxl").exists() {
        eprintln!("SKIP: ./jxl_from_tree and ./djxl required — run `make setup`");
        return;
    }

    let cases = [
        ("320x320", prog(320, 320, "")),
        ("1024x1024", prog(1024, 1024, "")),
        ("2048x2048", prog(2048, 2048, "")),
        ("2048x1152", prog(2048, 1152, "")),
        ("XYB+Sq 1024", prog(1024, 1024, "Squeeze\nXYB")),
    ];

    println!(
        "\n{:<14} {:>6} {:>8} {:>16} {:>11}",
        "case", "MP", "enc ms", "djxl dec ms", "roundtrip"
    );
    println!("{}", "-".repeat(60));

    for (name, program) in &cases {
        let t = Instant::now();
        let jxl = render::encode_jxl_from_tree(program).expect("encode");
        let enc_ms = t.elapsed().as_secs_f64() * 1000.0;

        let (_w, w, h) = codec::decode_jxl(&jxl, 0).expect("warmup decode");
        let _ = (_w, h);
        let mp = (w as f64 * h as f64) / 1_000_000.0;

        let mut dec = Vec::with_capacity(K);
        for _ in 0..K {
            let t = Instant::now();
            let _ = codec::decode_jxl(&jxl, 0).expect("decode");
            dec.push(t.elapsed());
        }
        let (dm, dmin) = mean_min(&dec);
        println!(
            "{:<14} {:>6.2} {:>8.1} {:>9.1}/{:<5.1} {:>10.1}",
            name, mp, enc_ms, dm, dmin, enc_ms + dm
        );
    }

    // ── Concurrency: production fires ~55 decodes/action. djxl is --num_threads=1,
    // so throughput should scale with cores. Compare 20 sequential vs 20 parallel.
    let jxl = render::encode_jxl_from_tree(&prog(1024, 1024, "")).expect("encode");
    const N: usize = 20;

    let t = Instant::now();
    for _ in 0..N {
        let _ = codec::decode_jxl(&jxl, 0).expect("seq decode");
    }
    let seq = t.elapsed().as_secs_f64() * 1000.0;

    let t = Instant::now();
    std::thread::scope(|s| {
        for _ in 0..N {
            let jxl = &jxl;
            s.spawn(move || {
                let _ = codec::decode_jxl(jxl, 0).expect("par decode");
            });
        }
    });
    let par = t.elapsed().as_secs_f64() * 1000.0;
    println!(
        "\nConcurrency (1024², N={N}):  sequential {seq:.0} ms   parallel {par:.0} ms   speedup {:.2}x",
        seq / par
    );

    // XYB+Squeeze sanity: should be libjxl-correct (~100,105,107), not jxl-oxide's old (53,39,64).
    let xyb = "Bitdepth 8\nOrientation 2\nRCT 14\nSqueeze\nXYB\n\nif W+N-NW > -127\n  - Gradient -2\n  if x > 192\n    - Weighted - 16\n    - AvgAll + 24\n";
    let jxl = render::encode_jxl_from_tree(xyb).expect("encode xyb");
    let (rgba, _, _) = codec::decode_jxl(&jxl, 0).expect("decode xyb");
    println!("XYB+Squeeze mean (libjxl-correct): {:?}", mean_rgb(&rgba));
}
