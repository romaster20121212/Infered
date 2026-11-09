 
 
 
 
 
 
 

use std::collections::HashSet;

 
pub type Cell = (i32, i32, i32);

 
fn step(cell: Cell, axis: usize, delta: i32) -> Cell {
    let mut c = [cell.0, cell.1, cell.2];
    c[axis] += delta;
    (c[0], c[1], c[2])
}

 
 
pub fn components(cells: &[Cell]) -> Vec<Vec<Cell>> {
    let set: HashSet<Cell> = cells.iter().copied().collect();
    let mut seen: HashSet<Cell> = HashSet::with_capacity(set.len());
    let mut groups = Vec::new();

    for &start in cells {
        if !seen.insert(start) {
            continue;
        }
        let mut group = Vec::new();
        let mut stack = vec![start];
        while let Some(cell) = stack.pop() {
            group.push(cell);
            for axis in 0..3 {
                for delta in [-1, 1] {
                    let next = step(cell, axis, delta);
                    if set.contains(&next) && seen.insert(next) {
                        stack.push(next);
                    }
                }
            }
        }
        groups.push(group);
    }
    groups
}

 
 
pub fn bounds(cells: &[Cell]) -> Option<([f64; 3], [f64; 3])> {
    let first = *cells.first()?;
    let mut min = [first.0, first.1, first.2];
    let mut max = min;
    for &(x, y, z) in cells {
        for (axis, value) in [x, y, z].into_iter().enumerate() {
            min[axis] = min[axis].min(value);
            max[axis] = max[axis].max(value);
        }
    }
    Some((min.map(|v| v as f64), max.map(|v| (v + 1) as f64)))
}

 
 
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Edge {
    pub axis: usize,
    pub origin: Cell,
}

impl Edge {
     
    pub fn endpoints(self) -> ([f64; 3], [f64; 3]) {
        let a = [
            self.origin.0 as f64,
            self.origin.1 as f64,
            self.origin.2 as f64,
        ];
        let mut b = a;
        b[self.axis] += 1.0;
        (a, b)
    }

     
    pub fn touching_cells(self) -> [Cell; 4] {
        let (u, v) = match self.axis {
            0 => (1, 2),
            1 => (0, 2),
            _ => (0, 1),
        };
         
        const RING: [(i32, i32); 4] = [(-1, -1), (0, -1), (0, 0), (-1, 0)];
        RING.map(|(du, dv)| {
            let mut c = [self.origin.0, self.origin.1, self.origin.2];
            c[u] += du;
            c[v] += dv;
            (c[0], c[1], c[2])
        })
    }
}

 
 
 
 
pub fn crease_edges(cells: &HashSet<Cell>) -> Vec<Edge> {
    let mut candidates: HashSet<Edge> = HashSet::new();
    for &(x, y, z) in cells {
        for a in 0..=1 {
            for b in 0..=1 {
                candidates.insert(Edge {
                    axis: 0,
                    origin: (x, y + a, z + b),
                });
                candidates.insert(Edge {
                    axis: 1,
                    origin: (x + a, y, z + b),
                });
                candidates.insert(Edge {
                    axis: 2,
                    origin: (x + a, y + b, z),
                });
            }
        }
    }

    candidates
        .into_iter()
        .filter(|edge| {
            let filled = edge.touching_cells().map(|c| cells.contains(&c));
            match filled.iter().filter(|&&f| f).count() {
                 
                1 | 3 => true,
                 
                2 => filled[0] == filled[2],
                _ => false,
            }
        })
        .collect()
}

 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Face {
    pub cell: Cell,
    pub axis: usize,
     
    pub plus: bool,
}

 
pub fn exposed_faces(cells: &HashSet<Cell>) -> Vec<Face> {
    let mut faces = Vec::new();
    for &cell in cells {
        for axis in 0..3 {
            for plus in [false, true] {
                let delta = if plus { 1 } else { -1 };
                if !cells.contains(&step(cell, axis, delta)) {
                    faces.push(Face { cell, axis, plus });
                }
            }
        }
    }
    faces
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(cells: &[Cell]) -> HashSet<Cell> {
        cells.iter().copied().collect()
    }

    #[test]
    fn components_split_touching_and_separate_blocks() {
         
        let cells = [(0, 0, 0), (1, 0, 0), (2, 1, 0), (9, 9, 9)];
        let mut sizes: Vec<usize> = components(&cells).iter().map(Vec::len).collect();
        sizes.sort_unstable();
        assert_eq!(sizes, vec![1, 1, 2]);
    }

    #[test]
    fn components_cover_every_cell_once() {
        let cells = [(0, 0, 0), (0, 1, 0), (0, 0, 1), (5, 5, 5)];
        let total: usize = components(&cells).iter().map(Vec::len).sum();
        assert_eq!(total, cells.len());
    }

    #[test]
    fn bounds_enclose_all_cells() {
        let (min, max) = bounds(&[(1, 2, 3), (-1, 4, 0)]).unwrap();
        assert_eq!(min, [-1.0, 2.0, 0.0]);
        assert_eq!(max, [2.0, 5.0, 4.0]);
        assert!(bounds(&[]).is_none());
    }

    #[test]
    fn single_block_has_its_twelve_edges() {
        assert_eq!(crease_edges(&set(&[(0, 0, 0)])).len(), 12);
    }

    #[test]
    fn two_touching_blocks_outline_without_the_shared_seam() {
         
         
        assert_eq!(crease_edges(&set(&[(0, 0, 0), (1, 0, 0)])).len(), 16);
    }

    #[test]
    fn two_touching_blocks_have_ten_exposed_faces() {
        assert_eq!(exposed_faces(&set(&[(0, 0, 0), (1, 0, 0)])).len(), 10);
    }

    #[test]
    fn touching_cells_are_the_four_blocks_around_the_edge() {
        let edge = Edge {
            axis: 0,
            origin: (0, 0, 0),
        };
        let cells = edge.touching_cells();
        assert!(cells.contains(&(0, -1, -1)));
        assert!(cells.contains(&(0, 0, 0)));
        assert_eq!(cells.len(), 4);
    }
}
