use geo::{Coord, Line, Within};

use crate::vtt::Coordinate;

/// A vector with a position in space (so not a real vector) used to track room walls
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Vector {
    pub x: f64,
    pub y: f64,
    pub xstart: f64,
    pub ystart: f64,
}

const RAD2DEG: f64 = 180. / std::f64::consts::PI;

impl Vector {
    /// Create a vectors from a line and a starting point, the vectors returned will be from the
    /// start point to the two end points of the line. If the start point is equal to the end or
    /// start point of the line it will return only one vector that does not have length 0
    pub fn from_line(line: &Line, start_point: &Coord) -> Vec<Self> {
        let mut vectorvec = Vec::new();
        let xstart = start_point.x;
        let ystart = start_point.y;
        let x = line.end.x - xstart;
        let y = line.end.y - ystart;
        if x > 1e-9 && y > 1e-9 {
            vectorvec.push(Vector {
                x,
                y,
                xstart,
                ystart,
            });
        }
        let x = line.start.x - xstart;
        let y = line.start.y - ystart;
        if x > 1e-9 && y > 1e-9 {
            vectorvec.push(Vector {
                x,
                y,
                xstart,
                ystart,
            });
        }
        vectorvec
    }

    pub fn from_points(start: &Coordinate, end: &Coordinate) -> Self {
        let x = end.x - start.x;
        let y = end.y - start.y;
        Self {
            x,
            y,
            xstart: start.x,
            ystart: start.y,
        }
    }

    /// Given an ordered vector of intersections that are assumed to lie on one line this function will
    /// return vectors going between these intersections
    pub fn from_intersections(intersections: Vec<Coordinate>) -> Vec<Self> {
        let mut vec = Vec::new();
        intersections.iter().enumerate().for_each(|(i, c)| {
            let point = match intersections.get(i + 1) {
                Some(p) => p,
                None => return,
            };
            vec.push(Self::from_points(c, point));
            vec.push(Self::from_points(point, c))
        });
        vec
    }

    /// Give the smallest non-zero angle of the reverse of self and the other vectors in clockwise
    /// direction. This function returns the vector with angle 0 if there are no other vectors
    ///   
    /// in essence, this function is used to follow walls in counterclockwise direction, where
    /// walls are represented as vectors.
    pub fn smallest_angle<'a>(&self, others: Vec<&'a Vector>) -> &'a Vector {
        let mut smallest_angle: Option<(&Vector, f64)> = None;
        let mut zero_angle: Option<&Vector> = None;
        for other in others {
            let x = (self.x * other.x) + (self.y * other.y);
            let y = (self.x * other.y) - (self.y * other.x);
            let new_angle = (y.atan2(x) * RAD2DEG) + 180.;
            if new_angle < 1e-9 {
                zero_angle = Some(other);
                continue;
            }
            if let Some((vector, smallest_angle)) = smallest_angle.as_mut() {
                if *smallest_angle > new_angle {
                    *vector = other;
                    *smallest_angle = new_angle;
                }
            } else {
                smallest_angle = Some((other, new_angle));
            }
        }
        if let Some((vector, _)) = smallest_angle {
            return vector;
        } else {
            return zero_angle.expect("There should be a zero_angle vector");
        }
    }

    /// Get the next vector from the planar graph
    pub fn next<'a>(&self, planar_graph: &'a Vec<Vector>) -> &'a Vector {
        let xend = self.xstart + self.x;
        let yend = self.ystart + self.y;
        let next_vectors: Vec<&Vector> = planar_graph
            .into_iter()
            .filter(|v| (v.xstart - xend).abs() < 1e-9 && (v.ystart - yend).abs() < 1e-9)
            .collect();
        let next = self.smallest_angle(next_vectors);
        next
    }

    /// Whether one vector is the inverse of another
    pub fn is_inverse(&self, other: &Vector) -> bool {
        let dot = (self.x * other.x) + (self.y * other.y);
        let cos_theta = dot / (self.len() * other.len());
        (cos_theta + 1.).abs() < 1e-9
    }

    /// Calculate length of vector
    pub fn len(&self) -> f64 {
        ((self.x * self.x) + (self.y * self.y)).sqrt()
    }

    /// Whether the given coordinate is on this vector
    pub fn contains(&self, coordinate: &Coord) -> bool {
        let start = Coord {
            x: self.xstart,
            y: self.ystart,
        };
        let end = Coord {
            x: self.xstart + self.x,
            y: self.ystart + self.y,
        };
        let line = Line::new(start, end);

        if coordinate.is_within(&line) {
            return true;
        }
        let start = Coordinate::from_coord(start);
        let end = Coordinate::from_coord(end);
        let coordinate = Coordinate::from_coord(*coordinate);
        return coordinate == start || coordinate == end;
    }

    pub fn start(&self) -> Coord {
        Coord {
            x: self.xstart,
            y: self.ystart,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smallest_angle_case1() {
        let v = Vector {
            x: 0.0,
            y: 1.0,
            xstart: 0.0,
            ystart: 0.0,
        };

        let v1 = Vector {
            x: 1.0,
            y: -1.0,
            xstart: 0.0,
            ystart: 0.0,
        };
        let v2 = Vector {
            x: 1.0,
            y: 1.0,
            xstart: 0.0,
            ystart: 0.0,
        };
        let v3 = Vector {
            x: 0.0,
            y: 2.0,
            xstart: 0.0,
            ystart: 0.0,
        };
        let v4 = Vector {
            x: -1.0,
            y: 1.0,
            xstart: 0.0,
            ystart: 0.0,
        };
        let v5 = Vector {
            x: 0.0,
            y: -1.0,
            xstart: 0.0,
            ystart: 0.0,
        };

        let others = vec![&v1, &v2, &v3, &v4, &v5];
        let result = v.smallest_angle(others);

        assert_eq!(result.x, v1.x);
        assert_eq!(result.y, v1.y);
    }

    #[test]
    fn test_only_zero_angle() {
        let v = Vector {
            x: 0.0,
            y: 1.0,
            xstart: 0.0,
            ystart: 0.0,
        };

        let v1 = Vector {
            x: 0.0,
            y: -1.0,
            xstart: 0.0,
            ystart: 0.0,
        };

        let others = vec![&v1];
        let result = v.smallest_angle(others);

        assert_eq!(result.x, v1.x);
        assert_eq!(result.y, v1.y);
    }

    #[test]
    fn test_smallest_angle_case2() {
        let v = Vector {
            x: 2.0,
            y: -1.0,
            xstart: 0.0,
            ystart: 0.0,
        };

        let v1 = Vector {
            x: 0.0,
            y: 1.0,
            xstart: 0.0,
            ystart: 0.0,
        };
        let v2 = Vector {
            x: 1.0,
            y: -2.0,
            xstart: 0.0,
            ystart: 0.0,
        };

        let others = vec![&v1, &v2];
        let result = v.smallest_angle(others);

        assert_eq!(result.x, v2.x);
        assert_eq!(result.y, v2.y);
    }

    #[test]
    fn test_smallest_angle_single_vector() {
        let v = Vector {
            x: 3.0,
            y: 4.0,
            xstart: 0.0,
            ystart: 0.0,
        };
        let only = Vector {
            x: -2.0,
            y: 1.0,
            xstart: 0.0,
            ystart: 0.0,
        };

        let result = v.smallest_angle(vec![&only]);
        assert_eq!(result.x, only.x);
        assert_eq!(result.y, only.y);
    }

    #[test]
    fn test_smallest_angle_same_direction() {
        let v = Vector {
            x: 1.0,
            y: 0.0,
            xstart: 0.0,
            ystart: 0.0,
        };

        let same1 = Vector {
            x: 2.0,
            y: 0.0,
            xstart: 0.0,
            ystart: 0.0,
        };
        let same2 = Vector {
            x: 10.0,
            y: 0.0,
            xstart: 0.0,
            ystart: 0.0,
        };

        // Both make the exact same angle: 0 degrees
        let result = v.smallest_angle(vec![&same1, &same2]);

        // Either result is technically correct — ensure it's one of them
        assert!(
            (result.x == same1.x && result.y == same1.y)
                || (result.x == same2.x && result.y == same2.y)
        );
    }

    #[test]
    fn test_smallest_angle_opposite_direction() {
        let v = Vector {
            x: 1.0,
            y: 0.0,
            xstart: 0.0,
            ystart: 0.0,
        };

        let opposite = Vector {
            x: -1.0,
            y: 0.0,
            xstart: 0.0,
            ystart: 0.0,
        };
        let slight_angle = Vector {
            x: -1.0,
            y: 1.0,
            xstart: 0.0,
            ystart: 0.0,
        };

        // Opposite direction should NOT be chosen if a better angle exists
        let result = v.smallest_angle(vec![&opposite, &slight_angle]);

        assert_eq!(result.x, slight_angle.x);
        assert_eq!(result.y, slight_angle.y);
    }

    #[test]
    fn test_smallest_angle_all_negative_coordinates() {
        let v = Vector {
            x: -2.0,
            y: -3.0,
            xstart: 0.0,
            ystart: 0.0,
        };

        let o1 = Vector {
            x: -3.0,
            y: -4.0,
            xstart: 0.0,
            ystart: 0.0,
        }; // similar direction
        let o2 = Vector {
            x: 3.0,
            y: -3.0,
            xstart: 0.0,
            ystart: 0.0,
        }; // different quadrant
        let o3 = Vector {
            x: -1.0,
            y: 0.0,
            xstart: 0.0,
            ystart: 0.0,
        };

        let result = v.smallest_angle(vec![&o1, &o2, &o3]);

        assert_eq!(result.x, o3.x);
        assert_eq!(result.y, o3.y);
    }

    #[test]
    fn test_smallest_angle_perpendicular_vs_diagonal() {
        let v = Vector {
            x: 0.0,
            y: 1.0,
            xstart: 0.0,
            ystart: 0.0,
        };

        let perpendicular = Vector {
            x: 1.0,
            y: 0.0,
            xstart: 0.0,
            ystart: 0.0,
        }; // 90°
        let diagonal = Vector {
            x: 1.0,
            y: 1.0,
            xstart: 0.0,
            ystart: 0.0,
        }; // 45°

        let result = v.smallest_angle(vec![&perpendicular, &diagonal]);

        assert_eq!(result.x, perpendicular.x);
        assert_eq!(result.y, perpendicular.y);
    }

    #[test]
    fn test_contains_point_on_segment() {
        let vector = Vector {
            xstart: 0.0,
            ystart: 0.0,
            x: 5.0,
            y: 5.0,
        };
        let point = Coord { x: 3.0, y: 3.0 };
        assert!(vector.contains(&point));
    }

    #[test]
    fn test_contains_point_at_start() {
        let vector = Vector {
            xstart: 1.0,
            ystart: 2.0,
            x: 4.0,
            y: 4.0,
        };
        let point = Coord { x: 1.0, y: 2.0 };
        assert!(vector.contains(&point));
    }

    #[test]
    fn test_contains_point_at_end() {
        let vector = Vector {
            xstart: 1.0,
            ystart: 2.0,
            x: 4.0,
            y: 4.0,
        };
        let point = Coord { x: 5.0, y: 6.0 };
        assert!(vector.contains(&point));
    }

    #[test]
    fn test_contains_point_outside_segment() {
        let vector = Vector {
            xstart: 0.0,
            ystart: 0.0,
            x: 5.0,
            y: 5.0,
        };
        let point = Coord { x: 6.0, y: 6.0 };
        assert!(!vector.contains(&point));
    }

    #[test]
    fn test_contains_point_not_collinear() {
        let vector = Vector {
            xstart: 0.0,
            ystart: 0.0,
            x: 5.0,
            y: 5.0,
        };
        let point = Coord { x: 3.0, y: 4.0 };
        assert!(!vector.contains(&point));
    }

    #[test]
    fn test_contains_horizontal_vector() {
        let vector = Vector {
            xstart: 2.0,
            ystart: 3.0,
            x: 5.0,
            y: 0.0,
        };
        let point_on = Coord { x: 4.0, y: 3.0 };
        let point_off = Coord { x: 7.0, y: 4.0 };
        assert!(vector.contains(&point_on));
        assert!(!vector.contains(&point_off));
    }

    #[test]
    fn test_contains_vertical_vector() {
        let vector = Vector {
            xstart: 1.0,
            ystart: 1.0,
            x: 0.0,
            y: 5.0,
        };
        let point_on = Coord { x: 1.0, y: 4.0 };
        let point_off = Coord { x: 2.0, y: 4.0 };
        assert!(vector.contains(&point_on));
        assert!(!vector.contains(&point_off));
    }

    #[test]
    fn test_inverse() {
        let v1 = Vector {
            xstart: 0.0,
            ystart: 0.0,
            x: 0.0,
            y: 5.0,
        };
        let v2 = Vector {
            xstart: 0.0,
            ystart: 0.0,
            x: 0.0,
            y: -5.0,
        };
        assert!(v1.is_inverse(&v2));
    }
}
