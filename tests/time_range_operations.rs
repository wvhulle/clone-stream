use std::time::Duration;

use forked_stream::TimeRange;
use tokio::time::Instant;

#[test]
fn test_time_range_duration() {
    let start = Instant::now();
    let end = start + Duration::from_secs(10);
    let range = TimeRange { start, end };

    assert_eq!(range.duration(), Duration::from_secs(10));
}

#[test]
fn test_split_range() {
    let start = Instant::now();
    let end = start + Duration::from_secs(10);
    let range = TimeRange { start, end };

    let split = range.split(5, 0.1);

    assert_eq!(split.len(), 5);
    assert!(split[0].start >= range.start);
    assert!(split[4].end <= range.end);

    // Split into 1 range, expect it to be the same range
    let split_single = range.split(1, 0.0);
    assert_eq!(split_single.len(), 1);
    assert_eq!(split_single[0].start, range.start);
    assert_eq!(split_single[0].end, range.end);
}

#[test]
#[should_panic(expected = "Cannot split into 0 ranges")]
fn test_split_zero_ranges() {
    let start = Instant::now();
    let end = start + Duration::from_secs(10);
    let range = TimeRange { start, end };

    let _ = range.split(0, 0.1);
}

#[test]
#[should_panic(expected = "Gap ratio must be between 0 and 1")]
fn test_split_invalid_gap_ratio() {
    let start = Instant::now();
    let end = start + Duration::from_secs(10);
    let range = TimeRange { start, end };

    let _ = range.split(5, 2.0); // Gap ratio > 1
}

#[test]
fn test_split_exact_gap_ratio() {
    let start = Instant::now();
    let end = start + Duration::from_secs(10);
    let range = TimeRange { start, end };

    let sub_ranges = range.split(5, 0.1);

    assert_eq!(sub_ranges.len(), 5);
    assert_eq!(sub_ranges[0].start, start);
    assert_eq!(sub_ranges[4].end, end);

    assert_eq!(
        sub_ranges[1].start - sub_ranges[0].end,
        range.duration().mul_f32(0.1) / 4
    );
}

#[test]
fn test_one_moment() {
    let start = Instant::now();
    let range = TimeRange { start, end: start };

    let moments_one = range.moments(1);
    assert_eq!(moments_one.len(), 1);
    assert_eq!(moments_one[0], range.start);
}

#[test]
fn test_two_moments() {
    let start = Instant::now();
    let end = start + Duration::from_secs(10);
    let range = TimeRange { start, end };

    // Test for moments, splitting into 3 moments
    let moments = range.moments(2);
    assert_eq!(moments.len(), 2);
    assert_eq!(moments[0], range.start);
    assert_eq!(moments[1], range.end);
}

#[test]
fn test_three_moments() {
    let start = Instant::now();
    let end = start + Duration::from_secs(10);
    let range = TimeRange { start, end };

    // Test for moments, splitting into 3 moments
    let moments = range.moments(3);
    assert_eq!(moments.len(), 3);
    assert_eq!(moments[0], range.start);
    assert_eq!(moments[2], range.end);
}

#[test]
fn test_inner_padding() {
    let start = Instant::now();
    let end = start + Duration::from_secs(10);
    let range = TimeRange { start, end };

    // Test inner method with 0.2 padding
    let inner = range.inner(0.2);
    let expected_start = start + Duration::from_secs(1);
    let expected_end = end - Duration::from_secs(1);

    assert_eq!(inner.start, expected_start);
    assert_eq!(inner.end, expected_end);
}

#[test]
fn test_within() {
    let outer_start = Instant::now();
    let outer_end = outer_start + Duration::from_secs(10);
    let outer_range = TimeRange {
        start: outer_start,
        end: outer_end,
    };

    let inner_start = outer_start + Duration::from_secs(5);
    let inner_end = inner_start + Duration::from_secs(1);
    let inner_range = TimeRange {
        start: inner_start,
        end: inner_end,
    };

    // range1 should be within range2
    assert!(inner_range.within(&outer_range));

    // range2 should not be within range1
    assert!(!outer_range.within(&inner_range));
}

#[test]
fn test_middle() {
    let start = Instant::now();
    let end = start + Duration::from_secs(10);
    let range = TimeRange { start, end };

    let middle = range.middle();
    let expected_middle = start + Duration::from_secs(5);

    assert_eq!(middle, expected_middle);
}
