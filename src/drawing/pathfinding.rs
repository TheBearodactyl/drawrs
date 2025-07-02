use crate::utils::geometry::Point;
use std::collections::{HashMap, HashSet};

pub fn find_next_point(
    current: Point,
    spatial_index: &HashMap<(i32, i32), Vec<Point>>,
    visited: &HashSet<Point>,
    max_distance: i32,
) -> Option<Point> {
    let grid_size = max_distance.max(1);
    let current_grid_x = current.x / grid_size;
    let current_grid_y = current.y / grid_size;

    let mut best_point = None;
    let mut best_distance = i32::MAX;

    for dx in -1..=1 {
        for dy in -1..=1 {
            let grid_key = (current_grid_x + dx, current_grid_y + dy);
            if let Some(points) = spatial_index.get(&grid_key) {
                for &point in points {
                    if !visited.contains(&point) {
                        let dist_sq = current.distance_squared(&point);
                        if dist_sq <= max_distance * max_distance && dist_sq < best_distance {
                            best_distance = dist_sq;
                            best_point = Some(point);
                        }
                    }
                }
            }
        }
    }

    best_point
}

pub fn trace_line(
    start: Point,
    spatial_index: &HashMap<(i32, i32), Vec<Point>>,
    visited: &mut HashSet<Point>,
    max_distance: i32,
) -> Vec<Point> {
    let mut line = vec![start];
    visited.insert(start);

    while let Some(next) =
        find_next_point(*line.last().unwrap(), spatial_index, visited, max_distance)
    {
        line.push(next);
        visited.insert(next);
    }

    line
}
