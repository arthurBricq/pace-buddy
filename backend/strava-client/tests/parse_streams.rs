use strava_client::types::parse_streams_response;

#[test]
fn test_parse_keyed_stream_response() {
    let json = std::fs::read_to_string("tests/fixtures/strava_streams_sample.json")
        .expect("fixture file should exist");

    let streams = parse_streams_response(&json).expect("should parse successfully");

    // Should have all 8 stream types
    let types: Vec<&str> = streams.iter().map(|s| s.stream_type.as_str()).collect();
    assert!(types.contains(&"time"), "missing time stream");
    assert!(types.contains(&"distance"), "missing distance stream");
    assert!(
        types.contains(&"velocity_smooth"),
        "missing velocity_smooth stream"
    );
    assert!(types.contains(&"heartrate"), "missing heartrate stream");
    assert!(types.contains(&"altitude"), "missing altitude stream");
    assert!(types.contains(&"moving"), "missing moving stream");
    assert!(types.contains(&"latlng"), "missing latlng stream");
    assert!(types.contains(&"cadence"), "missing cadence stream");

    // Check data is correctly extracted
    let time_stream = streams.iter().find(|s| s.stream_type == "time").unwrap();
    let time_data: Vec<i64> = serde_json::from_value(time_stream.data.clone()).unwrap();
    assert_eq!(time_data.len(), 10);
    assert_eq!(time_data[0], 0);
    assert_eq!(time_data[9], 9);

    // Check that parsed_type works for known types
    let velocity = streams
        .iter()
        .find(|s| s.stream_type == "velocity_smooth")
        .unwrap();
    assert!(velocity.parsed_type().is_some());

    // latlng should parse to a known type too
    let latlng = streams.iter().find(|s| s.stream_type == "latlng").unwrap();
    assert!(latlng.parsed_type().is_some());
}

#[test]
fn test_parse_stream_to_domain_conversion() {
    let json = std::fs::read_to_string("tests/fixtures/strava_streams_sample.json")
        .expect("fixture file should exist");

    let streams = parse_streams_response(&json).expect("should parse");
    let domain_streams = strava_client::strava_streams_to_domain(streams, uuid::Uuid::nil());

    // latlng is supported but all numeric types should convert
    assert!(
        domain_streams.len() >= 7,
        "Expected at least 7 domain streams, got {}",
        domain_streams.len()
    );

    // Check that data_json is valid JSON for each stream
    for ds in &domain_streams {
        let _: serde_json::Value =
            serde_json::from_str(&ds.data_json).expect("data_json should be valid JSON");
    }
}
