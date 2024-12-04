use geo::Line;

use crate::vtt::Coordinate;

/// Helper function: In essence this calculates the distance between a point and the max or minimum
/// boundary.
pub fn checked_div(numerator: f64, denominator: f64) -> Option<f64> {
    if denominator == 0.0 {
        return None;
    }
    let fraction = numerator / denominator;
    if fraction < 0.0 {
        return None;
    }
    Some(fraction)
}

/// Given a line_of_sight parameter this will return a Vec of all line segments
pub fn get_line_segments(line_of_sight_elements: Vec<Vec<Coordinate>>) -> Vec<Line> {
    let mut all_lines: Vec<Line> = Vec::new();
    for lines in line_of_sight_elements {
        let mut prev_coord: Option<Coordinate> = None;
        for coordinate in lines {
            if let Some(prev) = prev_coord.clone() {
                all_lines.push(Line::new(prev, coordinate.clone()));
            }
            prev_coord = Some(coordinate);
        }
    }
    all_lines
}

#[cfg(test)]
mod tests {
    use crate::helper::checked_div;
    use crate::helper::get_line_segments;
    use crate::vtt::Coordinate;
    use geo::Line;

    #[test]
    fn test_checked_div() {
        let cases = vec![
            (10.0, 2.0, Some(5.0)),
            (0.0, 2.0, Some(0.0)),
            (10.0, 0.0, None),
            (-10.0, 2.0, None),
            (10.0, -2.0, None),
            (-10.0, -2.0, Some(5.0)),
            (-5.0, 10.0, None),
            (5.0, 10.0, Some(0.5)),
            (1e10, 2e5, Some(50000.0)),
            (1e-10, 2e-5, Some(0.000005)), // Small numbers
        ];

        let epsilon = 1e-10; // Tolerance for floating-point comparison

        for (numerator, denominator, expected) in cases {
            let result = checked_div(numerator, denominator);

            match (result, expected) {
                (Some(actual), Some(expected_value)) => {
                    assert!(
                        (actual - expected_value).abs() < epsilon,
                        "Failed on input ({}, {}): expected approximately {:?}, got {:?}",
                        numerator,
                        denominator,
                        expected_value,
                        actual
                    );
                }
                (None, None) => {} // Both are None, so the test passes
                _ => panic!(
                    "Failed on input ({}, {}): expected {:?}, got {:?}",
                    numerator, denominator, expected, result
                ),
            }
        }
    }

    #[test]
    fn test_empty_input() {
        let input: Vec<Vec<Coordinate>> = vec![];
        let result = get_line_segments(input);
        assert_eq!(result, vec![], "Expected no lines for empty input");
    }

    #[test]
    fn test_single_segment() {
        let input = vec![vec![
            Coordinate { x: 0.0, y: 0.0 },
            Coordinate { x: 1.0, y: 1.0 },
        ]];
        let expected = vec![Line::new(
            Coordinate { x: 0.0, y: 0.0 },
            Coordinate { x: 1.0, y: 1.0 },
        )];
        let result = get_line_segments(input);
        assert_eq!(result, expected, "Expected a single line segment");
    }

    #[test]
    fn test_multiple_segments_in_one_list() {
        let input = vec![vec![
            Coordinate { x: 0.0, y: 0.0 },
            Coordinate { x: 1.0, y: 1.0 },
            Coordinate { x: 2.0, y: 2.0 },
        ]];
        let expected = vec![
            Line::new(Coordinate { x: 0.0, y: 0.0 }, Coordinate { x: 1.0, y: 1.0 }),
            Line::new(Coordinate { x: 1.0, y: 1.0 }, Coordinate { x: 2.0, y: 2.0 }),
        ];
        let result = get_line_segments(input);
        assert_eq!(result, expected, "Expected multiple segments from one list");
    }

    #[test]
    fn test_multiple_lists() {
        let input = vec![
            vec![Coordinate { x: 0.0, y: 0.0 }, Coordinate { x: 1.0, y: 1.0 }],
            vec![Coordinate { x: 2.0, y: 2.0 }, Coordinate { x: 3.0, y: 3.0 }],
        ];
        let expected = vec![
            Line::new(Coordinate { x: 0.0, y: 0.0 }, Coordinate { x: 1.0, y: 1.0 }),
            Line::new(Coordinate { x: 2.0, y: 2.0 }, Coordinate { x: 3.0, y: 3.0 }),
        ];
        let result = get_line_segments(input);
        assert_eq!(result, expected, "Expected segments from multiple lists");
    }

    #[test]
    fn test_single_point_list() {
        let input = vec![vec![Coordinate { x: 0.0, y: 0.0 }]];
        let result = get_line_segments(input);
        assert_eq!(
            result,
            vec![],
            "Expected no segments for a single-point list"
        );
    }

    #[test]
    fn test_mixed_lists() {
        let input = vec![
            vec![Coordinate { x: 0.0, y: 0.0 }, Coordinate { x: 1.0, y: 1.0 }],
            vec![Coordinate { x: 2.0, y: 2.0 }],
            vec![
                Coordinate { x: 3.0, y: 3.0 },
                Coordinate { x: 4.0, y: 4.0 },
                Coordinate { x: 5.0, y: 5.0 },
            ],
        ];
        let expected = vec![
            Line::new(Coordinate { x: 0.0, y: 0.0 }, Coordinate { x: 1.0, y: 1.0 }),
            Line::new(Coordinate { x: 3.0, y: 3.0 }, Coordinate { x: 4.0, y: 4.0 }),
            Line::new(Coordinate { x: 4.0, y: 4.0 }, Coordinate { x: 5.0, y: 5.0 }),
        ];
        let result = get_line_segments(input);
        assert_eq!(
            result, expected,
            "Expected segments for mixed lists with varying points"
        );
    }
}
