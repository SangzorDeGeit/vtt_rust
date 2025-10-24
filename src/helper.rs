use std::collections::HashMap;

use geo::LineIntersection::{Collinear, SinglePoint};
use geo::{line_intersection, Coord, Line, LineString, Polygon};

use crate::vtt::Coordinate;

const STEP_SIZE: f64 = 0.2;
// Floating point multiplier to avoid floating point arithmetic
const PRECISION: f64 = 10_000.0;

/// Generate a Polygon representing the area that the pov can see. This vision is
/// blocked by walls
pub fn calculate_direct_los(
    pov: Coordinate,
    wall_segments: &Vec<Line>,
    origin: &Coordinate,
    size: &Coordinate,
) -> Polygon {
    let mut top_intersections = Vec::new();
    let mut right_intersections = Vec::new();
    let mut bottom_intersections = Vec::new();
    let mut left_intersections = Vec::new();
    // These asserts will make sure that the following logic will not fall apart due to floating
    // point arithmetic
    assert!(origin.x >= 0.0, "Origin x must positive");
    assert!(origin.y >= 0.0, "Origin y must be positive");
    assert_eq!(size.x.fract(), 0.0, "The size must be a whole number");
    assert_eq!(size.y.fract(), 0.0, "The size must be a whole number");
    let x_min = origin.x as i32;
    let x_max = size.x as i32;
    let y_min = origin.y;
    let y_max = size.y;
    let start = Coord { x: pov.x, y: pov.y };
    // we do not loop through floats due to inaccuracies in floating point arithmetic
    // In the first loop we vary x and make a line for pov to the top and bottom of the map
    for x in x_min..=x_max * (1.0 / STEP_SIZE) as i32 {
        let x = f64::from(x) * STEP_SIZE;

        // Line from pov to top edge
        let mut end = Coord { x, y: y_min };
        let line = Line::new(start, end);
        let intersection =
            find_intersection(&line, wall_segments, 0).expect("Skip=0 cannot result in None value");
        top_intersections.push(intersection);

        // Line from pov to bottom edge
        end = Coord { x, y: y_max };
        let line = Line::new(start, end);
        let intersection =
            find_intersection(&line, wall_segments, 0).expect("Skip=0 cannot result in None value");
        bottom_intersections.push(intersection);
    }
    let x_min = origin.x;
    let x_max = size.x;
    let y_min = origin.y as i32;
    let y_max = size.y as i32;
    for y in y_min..=y_max * (1.0 / STEP_SIZE) as i32 {
        // Exclude the first and last iteration (already calculated in the previous loop)
        if y == 0 || y == y_max * (1.0 / STEP_SIZE) as i32 {
            continue;
        }
        let y = f64::from(y) * STEP_SIZE;

        // Line from pov to left edge
        let mut end = Coord { x: x_min, y };
        let line = Line::new(start, end);
        let intersection =
            find_intersection(&line, wall_segments, 0).expect("Skip=0 cannot result in None value");
        left_intersections.push(intersection);

        // Line from pov to right edge
        end = Coord { x: x_max, y };
        let line = Line::new(start, end);
        let intersection =
            find_intersection(&line, wall_segments, 0).expect("Skip=0 cannot result in None value");
        right_intersections.push(intersection);
    }
    // If we want to create a linestring in clockwise direction starting from 0.0:
    // reverse the left and bottom intersection vectors (left should go from bottom to top and
    // bottom should go from right to left)
    bottom_intersections.reverse();
    left_intersections.reverse();
    top_intersections.append(&mut right_intersections);
    top_intersections.append(&mut bottom_intersections);
    top_intersections.append(&mut left_intersections);
    let first = top_intersections.first().expect("No intersection found");
    let last = top_intersections.last().expect("No intersection found");
    // Make sure the ring is closed
    if distance(first, last) > 1e-9 {
        top_intersections.push(first.clone());
    }
    assert!(
        top_intersections.len() > 2,
        "Not enough intersections to form a linestring"
    );
    let los_ring = LineString::new(top_intersections);
    assert!(
        los_ring.is_closed(),
        "The resulting line of sight ring is not closed (Begin and end coordinate are not equal)"
    );
    let polygon = Polygon::new(los_ring, vec![]);
    polygon
}

/// Generate a linestring that will return the line of sight from the pov point, the pov can look
/// perfectly around walls.
/// Get all the intersection points with all the vectors going FROM the point
/// input an array of lines
/// compare line 1 with line 2, then 3 then 4 etc. get intersections
/// if two lines intersect, the intersection point is always closer to the starting point compared
/// to the end point
pub fn calculate_indirect_los(pov: Coordinate, wall_segments: &Vec<Line>) -> Polygon {
    todo!("Implement this function")
}

/// Given a line and an array of wall segments, this function will return the intersection point
/// closest to the start point of the line. the `skip` variable determines how many intersection points to skip
/// from closest to the start point of the line. The last intersection point will always be the end point of
/// the input line (i.e. the edge of the image). If this intersection point is logically skipped it will return None
pub fn find_intersection(line: &Line, wall_segments: &Vec<Line>, skip: usize) -> Option<Coord> {
    // distances times PRECISION: so PRECISION precision points per square
    let mut all_intersections: HashMap<i64, Coord> = HashMap::new();
    let mut distances: Vec<i64> = Vec::new();
    for segment in wall_segments {
        let intersection =
            match line_intersection::line_intersection(line.to_owned(), segment.to_owned()) {
                Some(i) => i,
                None => continue,
            };
        // The line intersects with a point on a wall segment
        if let SinglePoint { intersection, .. } = intersection {
            let distance = distance(&intersection, &line.start);
            let distance = (distance * PRECISION) as i64;
            all_intersections.insert(distance, intersection);
            distances.push(distance);
            continue;
        }
        // The line goes trough the start and end point of a wall segment
        if let Collinear {
            intersection: intersection_line,
        } = intersection
        {
            let distance_start = distance(&intersection_line.start, &line.start);
            let distance_end = distance(&intersection_line.end, &line.start);
            // colinearity may not mean that pov is on the wall segment but this is should be
            // tested before using this function

            if distance_start < distance_end {
                let distance = (distance_start * PRECISION) as i64;
                all_intersections.insert(distance, intersection_line.start);
                distances.push(distance);
            }
            if distance_start > distance_end {
                let distance = (distance_end * PRECISION) as i64;
                all_intersections.insert(distance, intersection_line.start);
                distances.push(distance);
            }
        }
    }
    // Add the edge intersection to the map and list
    let edge_distance = distance(&line.start, &line.end);
    let edge_distance = (edge_distance * PRECISION) as i64;
    distances.push(edge_distance);
    all_intersections.insert(edge_distance, line.end);

    distances.sort();
    let key = match distances.get(skip) {
        Some(k) => k,
        None => return None,
    };
    let intersection = all_intersections
        .get(key)
        .expect("Distance not found in the map");
    return Some(intersection.clone());
}

/// Calculates the distance between two points
fn distance(c1: &Coord, c2: &Coord) -> f64 {
    //sqrt[(|x1-x2|^2) + (|y1-y2|^2)]
    return ((c1.x - c2.x).abs().powi(2) + (c1.y - c2.y).abs().powi(2)).sqrt();
}

#[cfg(test)]
mod test_find_intersection {
    use super::*;

    fn create_coord(x: f64, y: f64) -> Coord {
        Coord { x, y }
    }

    fn create_line(start: Coord, end: Coord) -> Line {
        Line { start, end }
    }

    fn coord_eq(c1: Coord, c2: Coord) {
        let distance = distance(&c1, &c2);
        assert!(
            distance < 1e-9,
            "Coords c1:{:?} and c2:{:?} are not equal",
            c1,
            c2
        );
    }

    #[test]
    fn test_no_intersections() {
        let line = create_line(create_coord(0.0, 0.0), create_coord(5.0, 5.0));
        let wall_segments = vec![
            create_line(create_coord(10.0, 10.0), create_coord(15.0, 15.0)),
            create_line(create_coord(20.0, 20.0), create_coord(25.0, 25.0)),
        ];

        // There are no intersections, so return the end point of the line (5.0, 5.0)
        let result = find_intersection(&line, &wall_segments, 0);
        coord_eq(result.expect("result was None"), create_coord(5.0, 5.0));
    }

    #[test]
    fn test_no_intersection_skip() {
        let line = create_line(create_coord(0.0, 0.0), create_coord(5.0, 5.0));
        let wall_segments = vec![
            create_line(create_coord(10.0, 10.0), create_coord(15.0, 15.0)),
            create_line(create_coord(20.0, 20.0), create_coord(25.0, 25.0)),
        ];

        // There are no intersections and the first intersection should be skipped, so return None
        let result = find_intersection(&line, &wall_segments, 1);
        assert_eq!(result, None, "Result was not None: {:?}", result)
    }

    #[test]
    fn test_one_intersection() {
        let line = create_line(create_coord(0.0, 0.0), create_coord(5.0, 5.0));
        let wall_segments = vec![
            create_line(create_coord(1.0, 3.0), create_coord(3.0, 1.0)), // Only one intersection
        ];

        // Only one intersection, so return it
        let result = find_intersection(&line, &wall_segments, 0);
        coord_eq(result.expect("result was None"), create_coord(2.0, 2.0)); // The intersection point
    }

    #[test]
    fn test_multiple_intersection() {
        let line = create_line(create_coord(0.0, 0.0), create_coord(5.0, 5.0));
        let wall_segments = vec![
            create_line(create_coord(1.0, 0.0), create_coord(1.0, 6.0)), // Intersects
            create_line(create_coord(1.0, 3.0), create_coord(3.0, 1.0)), // Intersects
            create_line(create_coord(1.0, 0.0), create_coord(3.0, 0.0)), // Does not intersect
        ];

        // Two intersections
        let result = find_intersection(&line, &wall_segments, 0);
        coord_eq(result.expect("result was None"), create_coord(1.0, 1.0)); // First intersection point
        let result = find_intersection(&line, &wall_segments, 1);
        coord_eq(result.expect("result was None"), create_coord(2.0, 2.0)); // Second intersection point
        let result = find_intersection(&line, &wall_segments, 2);
        coord_eq(result.expect("result was None"), create_coord(5.0, 5.0)); // End of line
    }

    #[test]
    fn test_on_parallel_wall() {
        let line = create_line(create_coord(0.0, 0.0), create_coord(5.0, 5.0));
        let wall_segments = vec![
            create_line(create_coord(1.0, 1.0), create_coord(3.0, 3.0)), // Intersects
        ];

        let result = find_intersection(&line, &wall_segments, 0);
        coord_eq(result.expect("result was None"), create_coord(1.0, 1.0)); // Intersects with the
                                                                            // start of the line
    }
}

#[cfg(test)]
mod test_distance {
    use super::*;

    // Helper function to create Coord instances
    fn create_coord(x: f64, y: f64) -> Coord {
        Coord { x, y }
    }

    #[test]
    fn test_distance_origin_to_point() {
        let c1 = create_coord(0.0, 0.0); // Origin point (0, 0)
        let c2 = create_coord(3.0, 4.0); // Point (3, 4)

        let result = distance(&c1, &c2);
        assert!((result - 5.0).abs() < 1e-9); // Expected result is 5.0, from a 3-4-5 triangle.
    }

    #[test]
    fn test_distance_same_point() {
        let c1 = create_coord(2.0, 3.0); // Point (2, 3)
        let c2 = create_coord(2.0, 3.0); // Same point (2, 3)

        let result = distance(&c1, &c2);
        assert!((result - 0.0).abs() < 1e-9); // The distance should be 0.
    }

    #[test]
    fn test_distance_horizontal_line() {
        let c1 = create_coord(1.0, 2.0); // Point (1, 2)
        let c2 = create_coord(5.0, 2.0); // Point (5, 2) on the same horizontal line

        let result = distance(&c1, &c2);
        assert!((result - 4.0).abs() < 1e-9); // The distance should be 4.0 along the x-axis.
    }

    #[test]
    fn test_distance_vertical_line() {
        let c1 = create_coord(3.0, 1.0); // Point (3, 1)
        let c2 = create_coord(3.0, 6.0); // Point (3, 6) on the same vertical line

        let result = distance(&c1, &c2);
        assert!((result - 5.0).abs() < 1e-9); // The distance should be 5.0 along the y-axis.
    }

    #[test]
    fn test_distance_negative_coordinates() {
        let c1 = create_coord(-3.0, -4.0); // Point (-3, -4)
        let c2 = create_coord(0.0, 0.0); // Origin point (0, 0)

        let result = distance(&c1, &c2);
        assert!((result - 5.0).abs() < 1e-9); // Distance from (-3, -4) to (0, 0) is 5.0.
    }

    #[test]
    fn test_distance_floating_point_precision() {
        let c1 = create_coord(1.23456789, 4.56789012); // Point (1.23456789, 4.56789012)
        let c2 = create_coord(9.87654321, 0.12345678); // Point (9.87654321, 0.12345678)

        // Calculate the expected Euclidean via a calculator
        let expected_result = 9.7178559952899;
        let result = distance(&c1, &c2);
        assert!(
            (result - expected_result).abs() < 1e-9,
            "Expected: {}, found: {}",
            expected_result,
            result
        ); // Allowing a small tolerance due to floating point precision.
    }
}
