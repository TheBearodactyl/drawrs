use crate::drawing::pathfinding::trace_line;
use crate::utils::geometry::Point;
use std::collections::{HashMap, HashSet};

pub fn build_spatial_index(
    points: &HashSet<Point>,
    grid_size: i32,
) -> HashMap<(i32, i32), Vec<Point>> {
    let mut spatial_index: HashMap<(i32, i32), Vec<Point>> = HashMap::new();

    for &point in points {
        let grid_x = point.x / grid_size;
        let grid_y = point.y / grid_size;
        spatial_index
            .entry((grid_x, grid_y))
            .or_default()
            .push(point);
    }

    spatial_index
}

pub fn find_connected_components(points: HashSet<Point>, max_distance: i32) -> Vec<Vec<Point>> {
    let spatial_index = build_spatial_index(&points, max_distance.max(1));
    let mut visited = HashSet::new();
    let mut lines = Vec::new();

    let mut sorted_points: Vec<_> = points.into_iter().collect();
    sorted_points.sort_by_key(|p| (p.y, p.x));

    for start_point in sorted_points {
        if !visited.contains(&start_point) {
            let line = trace_line(start_point, &spatial_index, &mut visited, max_distance);

            if line.len() > 2 {
                lines.push(line);
            }
        }
    }

    lines.sort_by_key(|line| std::cmp::Reverse(line.len()));
    lines
}
