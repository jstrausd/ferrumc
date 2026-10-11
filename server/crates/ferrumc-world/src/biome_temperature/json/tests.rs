use super::*;

const VALID: &str = r#"{"temperature":0.5,"has_precipitation":true}"#;

fn with_extra(extra: &str) -> String {
    format!(r#"{{"temperature":0.5,"has_precipitation":true,"extra":{extra}}}"#)
}

#[test]
fn full_json_grammar_and_skipped_payloads_fail_closed() {
    for input in [
        "",
        "null",
        "[]",
        "0",
        "{",
        "{\"a\"}",
        "{\"a\":}",
        "{\"a\":0,}",
        "{\"a\":0 \"b\":1}",
        "{\"a\":01}",
        "{\"a\":-}",
        "{\"a\":+1}",
        "{\"a\":1.}",
        "{\"a\":.1}",
        "{\"a\":1e}",
        "{\"a\":1e+}",
        "{\"a\":NaN}",
        "{\"a\":Infinity}",
        "{\"a\":tru}",
        "{\"a\":falsee}",
        "{\"a\":[1,]}",
        "{\"a\":[,1]}",
        "{\"a\":[1 2]}",
        "{\"a\":\"unterminated}",
        "{\"a\":\"raw\nnewline\"}",
        "{\"a\":\"raw\tcontrol\"}",
        "{\"a\":\"\0\"}",
    ] {
        assert!(parse(input).is_err(), "{input:?}");
    }
    for extra in [
        "01",
        "-01",
        "1e-",
        "[true,]",
        "{\"x\":0,}",
        "null null",
        "\u{a0}0",
    ] {
        assert!(
            parse(&with_extra(extra)).is_err(),
            "skipped invalid {extra}"
        );
    }
    for tail in ["0", "x", "{}", "\u{a0}", "\u{b}"] {
        assert!(parse(&format!("{VALID}{tail}")).is_err());
    }
    for extra in [
        "null",
        "false",
        "true",
        "[]",
        "{}",
        "[-0,-0.1,0,1.25e+3,1E-3]",
        "1e100000",
        r#"{"nested":[null,true,"Grüße🌍",{"x":-0}]}"#,
    ] {
        assert!(parse(&with_extra(extra)).is_ok(), "{extra}");
    }
    assert!(parse(&format!(" \t\r\n{VALID}\n\r\t ")).is_ok());
}

#[test]
fn strict_unicode_escape_decoding_and_duplicate_decoded_key_identity() {
    for extra in [
        r#""\ud83d\ude00""#,
        r#""Grüße🌍""#,
        r#""\"\\\/\b\f\n\r\t""#,
        r#""\u0000\u007f\u0080\u07ff\u0800\uffff""#,
    ] {
        assert!(parse(&with_extra(extra)).is_ok(), "{extra}");
    }
    for extra in [
        r#""\ud800""#,
        r#""\udc00""#,
        r#""\ud800\u0000""#,
        r#""\ud800\ud800""#,
        r#""\u000g""#,
        r#""\u123""#,
        r#""\x00""#,
        r#""\v""#,
        r#""\U00000041""#,
        r#""\ud800x""#,
    ] {
        assert!(
            matches!(
                parse(&with_extra(extra)).unwrap_err().kind(),
                Kind::InvalidJson
            ),
            "{extra}"
        );
    }
    for input in [
        r#"{"temperature":0,"\u0074emperature":1,"has_precipitation":true}"#,
        r#"{"temperature":0,"has_precipitation":true,"x":0,"x":1}"#,
        r#"{"temperature":0,"has_precipitation":true,"x":{"é":0,"\u00e9":1}}"#,
        r#"{"temperature":0,"has_precipitation":true,"x":{"😀":0,"\ud83d\ude00":1}}"#,
        r#"{"temperature":0,"has_precipitation":true,"x":{"a\/b":0,"a/b":1}}"#,
        r#"{"temperature":0,"has_precipitation":true,"x":{"\u0000":0,"\u0000":1}}"#,
    ] {
        let error = parse(input).unwrap_err();
        assert_eq!(error.kind(), &Kind::DuplicateField, "{input}");
        assert!(error.offset().is_some());
    }
    assert!(parse(&with_extra(
        r#"[{"x":0},{"x":1},{"a":{"x":0},"b":{"x":1}}]"#
    ))
    .is_ok());
    let escaped = r#"{"\u0074emperature":0.5,"has_\u0070recipitation":false,"temperature_modifier":"\u0066rozen"}"#;
    let (climate, _) = parse(escaped).unwrap();
    assert_eq!(
        climate,
        Climate {
            temperature: 0.5,
            has_precipitation: false,
            frozen: true
        }
    );
}

#[test]
fn climate_presence_types_values_and_direct_f32_rounding_are_checked() {
    for input in ["{}", "{\"temperature\":0}", "{\"has_precipitation\":true}"] {
        let error = parse(input).unwrap_err();
        assert_eq!(error.kind(), &Kind::MissingClimateField);
        assert!(matches!(
            error.field(),
            Some("temperature" | "has_precipitation")
        ));
        assert!(error.offset().is_some());
    }
    for value in ["null", "true", "false", "\"0.5\"", "[]", "{}", "1e10000"] {
        let input = format!(r#"{{"temperature":{value},"has_precipitation":true}}"#);
        let error = parse(&input).unwrap_err();
        assert_eq!(error.kind(), &Kind::InvalidClimateField);
        assert_eq!(error.field(), Some("temperature"));
    }
    for value in ["0", "1", "null", "\"true\"", "[]", "{}"] {
        let input = format!(r#"{{"temperature":0,"has_precipitation":{value}}}"#);
        let error = parse(&input).unwrap_err();
        assert_eq!(error.kind(), &Kind::InvalidClimateField);
        assert_eq!(error.field(), Some("has_precipitation"));
    }
    for value in [
        "0",
        "true",
        "null",
        "[]",
        "{}",
        "\"NONE\"",
        "\"unknown\"",
        "\"frozen \"",
    ] {
        let input = format!(
            r#"{{"temperature":0,"has_precipitation":true,"temperature_modifier":{value}}}"#
        );
        let error = parse(&input).unwrap_err();
        assert_eq!(error.kind(), &Kind::InvalidClimateField);
        assert_eq!(error.field(), Some("temperature_modifier"));
    }
    for (modifier, frozen) in [("none", false), ("frozen", true)] {
        let input = format!(
            r#"{{"temperature":0,"has_precipitation":true,"temperature_modifier":"{modifier}"}}"#
        );
        assert_eq!(parse(&input).unwrap().0.frozen, frozen);
    }
    assert!(!parse(VALID).unwrap().0.frozen);
    for token in ["-0", "-0.0"] {
        let input = format!(r#"{{"temperature":{token},"has_precipitation":true}}"#);
        assert_eq!(
            parse(&input).unwrap().0.temperature.to_bits(),
            (-0.0_f32).to_bits()
        );
    }
    // A parser-policy control, not fabricated Original temperature evidence.
    let token = "1.00000005960464477539062500000000000000000000000001";
    let input = format!(r#"{{"temperature":{token},"has_precipitation":true}}"#);
    let actual = parse(&input).unwrap().0.temperature;
    assert_eq!(actual.to_bits(), token.parse::<f32>().unwrap().to_bits());
    assert_ne!(
        actual.to_bits(),
        (token.parse::<f64>().unwrap() as f32).to_bits()
    );
}

fn refusal(input: &str, limit: Limit, count: usize, maximum: usize) {
    assert_eq!(
        parse(input).unwrap_err().kind(),
        &Kind::Limit {
            limit,
            count,
            maximum
        }
    );
}

#[test]
fn byte_node_depth_object_array_and_utf8_limits_at_n_minus_one_n_n_plus_one() {
    for bytes in [SOURCE_MAX - 1, SOURCE_MAX, SOURCE_MAX + 1] {
        let source = format!("{VALID}{}", " ".repeat(bytes - VALID.len()));
        if bytes <= SOURCE_MAX {
            assert!(parse(&source).is_ok());
        } else {
            refusal(&source, Limit::SourceBytes, bytes, SOURCE_MAX);
        }
    }
    for arrays in [30, 31, 32] {
        let source = with_extra(&format!("{}0{}", "[".repeat(arrays), "]".repeat(arrays)));
        if arrays < 32 {
            assert_eq!(parse(&source).unwrap().1.depth, arrays + 1);
        } else {
            refusal(&source, Limit::JsonDepth, 33, 32);
        }
    }
    for count in [255, 256, 257] {
        let array = with_extra(&format!("[{}]", vec!["0"; count].join(",")));
        let object = format!(
            r#"{{"temperature":0,"has_precipitation":true,{}}}"#,
            (0..count - 2)
                .map(|i| format!("\"k{i}\":0"))
                .collect::<Vec<_>>()
                .join(",")
        );
        for input in [array, object] {
            if count <= 256 {
                assert!(parse(&input).is_ok());
            } else {
                refusal(&input, Limit::ContainerEntries, 257, 256);
            }
        }
        for text in [
            "x".repeat(count),
            format!("{}{}", "😀".repeat(count / 4), "x".repeat(count % 4)),
        ] {
            let string = with_extra(&serde_json::to_string(&text).unwrap());
            let key = format!(
                r#"{{"temperature":0,"has_precipitation":true,{}:0}}"#,
                serde_json::to_string(&text).unwrap()
            );
            for input in [string, key] {
                if count <= 256 {
                    assert!(parse(&input).is_ok());
                } else {
                    refusal(&input, Limit::StringBytes, 257, 256);
                }
            }
        }
    }
    for nodes in [4095, 4096, 4097] {
        let arrays = (0..16)
            .map(|i| {
                format!(
                    "[{}]",
                    vec!["0"; if i == 15 { nodes - 23 - 15 * 255 } else { 255 }].join(",")
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let input = with_extra(&format!("[{arrays}]"));
        if nodes <= 4096 {
            assert_eq!(parse(&input).unwrap().1.nodes, nodes);
        } else {
            refusal(&input, Limit::JsonNodes, 4097, 4096);
        }
    }
    for bytes in [255, 256, 257] {
        let input = with_extra(&format!("\"{}\"", "\\u0061".repeat(bytes)));
        if bytes <= 256 {
            assert!(parse(&input).is_ok());
        } else {
            refusal(&input, Limit::StringBytes, 257, 256);
        }
    }
}
