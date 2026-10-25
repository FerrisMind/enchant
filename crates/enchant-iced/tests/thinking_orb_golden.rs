//! Numeric parity with the reference library.
//!
//! `tests/fixtures/thinking-orbs-golden.json` is the reference engine's own
//! output (`spec/orbs-golden.json` upstream): for every state at sizes 64 and
//! 20 and four times, every dot and line of the frame, plus the resolved
//! preset options. The port must reproduce them to the fixture's tolerance.

use enchant_iced::{OrbKnob, OrbSize, OrbState, ThinkingOrb};
use serde_json::Value;

fn golden() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/thinking-orbs-golden.json"
    );
    let text = std::fs::read_to_string(path).expect("read golden fixture");
    serde_json::from_str(&text).expect("parse golden fixture")
}

fn size_of(px: u64) -> OrbSize {
    match px {
        64 => OrbSize::Large,
        32 => OrbSize::Compact,
        20 => OrbSize::Inline,
        other => panic!("unexpected golden size {other}"),
    }
}

fn floats(v: &Value) -> Vec<f64> {
    v.as_array()
        .expect("array")
        .iter()
        .map(|x| x.as_f64().expect("number"))
        .collect()
}

#[test]
fn resolved_presets_match_the_reference() {
    let g = golden();
    let tolerance = g["tolerance"].as_f64().unwrap();
    let resolved = g["resolved"].as_object().unwrap();
    assert_eq!(resolved.len(), 18);

    for (key, entry) in resolved {
        let (state_name, px) = key.rsplit_once('-').unwrap();
        let state = OrbState::from_name(state_name).unwrap();
        let orb = ThinkingOrb::new()
            .state(state)
            .size(size_of(px.parse().unwrap()));
        let ours = orb.resolved_options();

        let speed = entry["speed"].as_f64().unwrap();
        assert!(
            (orb.engine_time(1.0) - speed).abs() < 1e-9,
            "{key}: preset speed"
        );

        let want = entry["opts"].as_object().unwrap();
        for (name, value) in want {
            let knob =
                OrbKnob::from_name(name).unwrap_or_else(|| panic!("{key}: unknown knob {name}"));
            let got = ours
                .get(knob)
                .unwrap_or_else(|| panic!("{key}: knob {name} missing"));
            assert!(
                (got - value.as_f64().unwrap()).abs() <= tolerance,
                "{key}: {name} = {got}, reference {value}"
            );
        }
        assert_eq!(
            ours.iter().count(),
            want.len(),
            "{key}: we resolve extra knobs the reference does not"
        );
    }
}

#[test]
fn frames_match_the_reference_at_every_golden_time() {
    let g = golden();
    let tolerance = g["tolerance"].as_f64().unwrap();
    let cases = g["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 72);

    let mut dots_checked = 0usize;
    let mut lines_checked = 0usize;
    for case in cases {
        let key = case["key"].as_str().unwrap();
        let state = OrbState::from_name(case["state"].as_str().unwrap()).unwrap();
        let size = size_of(case["size"].as_u64().unwrap());
        let t = case["t"].as_f64().unwrap();

        let frame = ThinkingOrb::new().state(state).size(size).frame_at(t);

        assert_eq!(
            frame.dots().len() as u64,
            case["dotCount"].as_u64().unwrap(),
            "{key}: dot count"
        );
        assert_eq!(
            frame.lines().len() as u64,
            case["lineCount"].as_u64().unwrap(),
            "{key}: line count"
        );

        // dot stride 6: x, y, z, r, white, a
        let want: Vec<[f64; 6]> = floats(&case["dots"])
            .chunks_exact(6)
            .map(|c| [c[0], c[1], c[2], c[3], c[4], c[5]])
            .collect();
        let got: Vec<[f64; 6]> = frame
            .dots()
            .iter()
            .map(|d| [d.x(), d.y(), d.z(), d.r(), d.white(), d.alpha()])
            .collect();
        for (i, (g, w)) in got.iter().zip(&want).enumerate() {
            for (c, (g, w)) in g.iter().zip(w).enumerate() {
                assert!(
                    (g - w).abs() <= tolerance,
                    "{key}: dot {i} field {c}: {g} vs {w}"
                );
            }
            dots_checked += 1;
        }

        // line stride 7: x1, y1, x2, y2, white, a, w
        let want = floats(&case["lines"]);
        for (i, (line, w)) in frame.lines().iter().zip(want.chunks_exact(7)).enumerate() {
            let (x1, y1) = line.start();
            let (x2, y2) = line.end();
            let got = [x1, y1, x2, y2, line.white(), line.alpha(), line.width()];
            for (c, (g, w)) in got.iter().zip(w).enumerate() {
                assert!(
                    (g - w).abs() <= tolerance,
                    "{key}: line {i} field {c}: {g} vs {w}"
                );
            }
            lines_checked += 1;
        }
    }
    assert!(dots_checked > 10_000, "only {dots_checked} dots compared");
    assert!(lines_checked > 100, "only {lines_checked} lines compared");
}
