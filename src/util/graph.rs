use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::ops::Add;

use crate::util::tree_seq::TreeSeq;

#[derive(Eq, Ord, PartialEq, PartialOrd, Hash, Copy, Clone)]
pub struct Vertex(usize);

#[derive(Eq, PartialEq, Clone)]
struct VertexEntry<VL, EL: Clone> {
    label: VL,
    neighbors: HashMap<Vertex, EL>,
}

#[derive(Eq, PartialEq, Clone)]
pub struct Graph<VL, EL: Clone> {
    table: HashMap<Vertex, VertexEntry<VL, EL>>,
    next_vertex: Vertex,
}

impl<VL, EL: Clone> Graph<VL, EL> {
    pub fn new() -> Self {
        Graph {
            table: HashMap::new(),
            next_vertex: Vertex(0),
        }
    }
    pub fn add_vertex(&mut self, label: VL) -> Vertex {
        let vertex = self.next_vertex;
        self.table.insert(self.next_vertex,
                          VertexEntry { label: label, neighbors: HashMap::new() });
        self.next_vertex.0 += 1;
        vertex
    }
    pub fn remove_vertex(&mut self, vertex: Vertex) -> (VL, Vec<(Vertex, EL)>) {
        let neighbors = self.table.remove(&vertex).unwrap();
        for (&neighbor, _) in neighbors.neighbors.iter() {
            self.table.get_mut(&neighbor).unwrap().neighbors.remove(&vertex);
        }
        (neighbors.label, neighbors.neighbors.into_iter().collect())
    }
    fn add_directed_edge(&mut self, v1: Vertex, v2: Vertex, label: EL) {
        self.table.get_mut(&v1).unwrap().neighbors.insert(v2, label);
    }
    pub fn add_edge(&mut self, v1: Vertex, v2: Vertex, label: EL) {
        self.add_directed_edge(v1, v2, label.clone());
        self.add_directed_edge(v2, v1, label);
    }
    fn remove_directed_edge(&mut self, v1: Vertex, v2: Vertex) -> Option<EL> {
        self.table.get_mut(&v1).unwrap().neighbors.remove(&v2)
    }
    pub fn remove_edge(&mut self, v1: Vertex, v2: Vertex) -> Option<EL> {
        match (self.remove_directed_edge(v1, v2), self.remove_directed_edge(v2, v1)) {
            (None, None) => None,
            (Some(x), Some(_)) => Some(x),
            _ => panic!(),
        }
    }
    pub fn get_edge(&self, v1: Vertex, v2: Vertex) -> Option<&EL> {
        self.table.get(&v1).unwrap().neighbors.get(&v2)
    }
    pub fn merge<FV, FE>(&mut self, v1: Vertex, v2: Vertex, mut fv: FV, mut fe: FE) -> Vertex
        where FV: FnMut(VL, VL, Option<EL>) -> VL,
              FE: FnMut(EL, EL) -> EL {
        let edge_label = self.remove_edge(v1, v2);
        let (v1l, v1es) = self.remove_vertex(v1);
        let (v2l, v2es) = self.remove_vertex(v2);
        let new_vertex = self.add_vertex(fv(v1l, v2l, edge_label));
        let mut union: HashMap<Vertex, EL> = v1es.into_iter().collect();
        for (neighbor, el2) in v2es {
            match union.remove(&neighbor) {
                None => { union.insert(neighbor, el2); }
                Some(el1) => { union.insert(neighbor, fe(el1, el2)); }
            }
        }
        for (neighbor, el) in union {
            self.add_edge(new_vertex, neighbor, el);
        }
        new_vertex
    }
    pub fn label(&self, vertex: Vertex) -> &VL {
        &self.table[&vertex].label
    }
    pub fn vertices(&self) -> impl Iterator<Item=(Vertex, &VL)> {
        self.table.iter().map(|(&k, e)| (k, &e.label))
    }
    pub fn debug_vertices(&self) -> Vec<(Vertex, VL)> where VL: Clone {
        let mut result: Vec<(Vertex, VL)> = self.vertices().map(|(v, l)| (v, l.clone())).collect();
        result.sort_by_key(|(v, _)| *v);
        result
    }
    pub fn num_vertices(&self) -> usize {
        self.table.len()
    }
    pub fn neighbors(&self, vertex: Vertex) -> impl Iterator<Item=(Vertex, &EL)> {
        self.table.get(&vertex).unwrap().neighbors.iter().map(|(&v, l)| (v, l))
    }
    pub fn debug_neighbors(&self, vertex: Vertex) -> Vec<(Vertex, EL)> {
        let mut result: Vec<(Vertex, EL)> = self.neighbors(vertex).map(|(v, l)| (v, l.clone())).collect();
        result.sort_by_key(|(v, _)| *v);
        result
    }
    pub fn map<VL2, EL2: Clone,
        FV: FnMut(Vertex, &VL) -> VL2,
        FE: FnMut(Vertex, Vertex, &EL) -> EL2>(&self, mut fv: FV, mut fe: FE) -> Graph<VL2, EL2> {
        Graph {
            table: self.table.iter().map(
                |(&v1, e)|
                    (v1, VertexEntry {
                        label: fv(v1, &e.label),
                        neighbors: e.neighbors.iter().map(|(&v2, e)| (v2, fe(v1, v2, e))).collect(),
                    })).collect(),
            next_vertex: self.next_vertex,
        }
    }
}

impl Graph<usize, ()> {
    pub fn simple(num_vs: usize, es: &[(usize, usize)]) -> Self {
        let mut graph = Graph::new();
        let vs: Vec<Vertex> = (0..num_vs).map(|n| graph.add_vertex(n)).collect();
        for &(v1, v2) in es {
            graph.add_edge(vs[v1], vs[v2], ());
        }
        graph
    }
}

impl fmt::Debug for Vertex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "V{:?}", self.0)
    }
}

impl<VL: fmt::Debug, EL: Clone + fmt::Debug> fmt::Debug for Graph<VL, EL> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut map = f.debug_map();
        for (v, VertexEntry { label, neighbors }) in self.table.iter() {
            map.entry(&(v, label), neighbors);
        }
        map.finish()
    }
}

fn minimum_cut_phase(graph: &Graph<TreeSeq<Vertex>, usize>) -> (Vertex, Vertex) {
    let mut remaining = priority_queue::PriorityQueue::new();
    for (vertex, _) in graph.vertices() {
        remaining.push(vertex, (0, vertex));
    }
    while remaining.len() > 2 {
        let (next, _) = remaining.pop().unwrap();
        for (neighbor, &weight) in graph.neighbors(next) {
            remaining.change_priority_by(&neighbor, |(old_weight, _)| (old_weight + weight, neighbor));
        }
    }
    (remaining.pop().unwrap().0, remaining.pop().unwrap().0)
}

pub fn stoer_wagner<VL, EL: Clone>(original: &Graph<VL, EL>) -> (usize, Vec<Vertex>) {
    assert!(original.num_vertices() >= 2);
    let mut graph: Graph<TreeSeq<Vertex>, usize> = original.map(|v, _| TreeSeq::Leaf(v), |_, _, _| 1);
    let mut best_weight = usize::max_value();
    let mut best_result = None;

    while graph.num_vertices() > 1 {
        let (v1, v2) = minimum_cut_phase(&graph);
        let weight: usize = graph.neighbors(v2).map(|(_, &w)| w).sum();
        if weight <= best_weight {
            best_weight = weight;
            best_result = Some(graph.label(v2).iter().cloned().collect())
        }
        graph.merge(v1, v2,
                    |v1, v2, _| v1.concat(v2),
                    |e1, e2| e1 + e2);
    }
    (best_weight, best_result.unwrap())
}

#[test]
fn test_merge() {
    {
        let mut graph: Graph<&'static str, &'static str> = Graph::new();
        let va = graph.add_vertex("a");
        let vb = graph.add_vertex("b");
        let vc = graph.merge(va, vb,
                             |v1, v2, e| {
                                 assert_eq!(v1, "a");
                                 assert_eq!(v2, "b");
                                 assert_eq!(e, None);
                                 "c"
                             },
                             |e1, e2| unreachable!());
        assert_eq!(graph.debug_vertices(), vec![(vc, "c")]);
        assert_eq!(graph.debug_neighbors(vc), vec![]);
    }
    {
        let mut graph: Graph<&'static str, &'static str> = Graph::new();
        let va = graph.add_vertex("a");
        let vb = graph.add_vertex("b");
        let ex = graph.add_edge(va, vb, "x");
        let vc = graph.merge(va, vb,
                             |v1, v2, e| {
                                 assert_eq!(v1, "a");
                                 assert_eq!(v2, "b");
                                 assert_eq!(e, Some("x"));
                                 "c"
                             },
                             |e1, e2| unreachable!());
        assert_eq!(graph.debug_vertices(), vec![(vc, "c")]);
        assert_eq!(graph.debug_neighbors(vc), vec![]);
    }
    {
        let mut graph: Graph<&'static str, &'static str> = Graph::new();
        let va = graph.add_vertex("a");
        let vb = graph.add_vertex("b");
        let vc = graph.add_vertex("c");
        let vd = graph.add_vertex("d");
        let ve = graph.add_vertex("e");
        let eab = graph.add_edge(va, vb, "ab");
        graph.add_edge(va, vc, "ac");
        graph.add_edge(va, vd, "ad");
        graph.add_edge(vb, vd, "bd");
        graph.add_edge(vb, ve, "be");
        let vab = graph.merge(va, vb,
                              |v1, v2, e| {
                                  assert_eq!(v1, "a");
                                  assert_eq!(v2, "b");
                                  assert_eq!(e, Some("ab"));
                                  "a+b"
                              },
                              |e1, e2| {
                                  assert_eq!(e1, "ad");
                                  assert_eq!(e2, "bd");
                                  "ad+bd"
                              });
        println!("{:?}", graph);
        assert_eq!(graph.debug_vertices(), vec![(vc, "c"), (vd, "d"), (ve, "e"), (vab, "a+b")]);
        assert_eq!(graph.debug_neighbors(vc), vec![(vab, "ac")]);
        assert_eq!(graph.debug_neighbors(vd), vec![(vab, "ad+bd")]);
        assert_eq!(graph.debug_neighbors(ve), vec![(vab, "be")]);
        assert_eq!(graph.debug_neighbors(vab), vec![(vc, "ac"), (vd, "ad+bd"), (ve, "be")]);
    }
}

#[test]
fn test_stoer_wagner() {
    fn run(graph: Graph<usize, ()>) -> (usize, Vec<usize>) {
        let (weight, vec) = stoer_wagner(&graph);
        let mut vec: Vec<usize> = vec.iter().map(|&v| *graph.label(v)).collect();
        vec.sort();
        (weight, vec)
    }

    assert_eq!((0, vec![0]), run(Graph::simple(2, &[])));
    assert_eq!((1, vec![0]), run(Graph::simple(2, &[(0, 1)])));

    assert_eq!((0, vec![2]), run(Graph::simple(3, &[])));
    assert_eq!((0, vec![2]), run(Graph::simple(3, &[(0, 1)])));
    assert_eq!((0, vec![1]), run(Graph::simple(3, &[(0, 2)])));
    assert_eq!((0, vec![0]), run(Graph::simple(3, &[(1, 2)])));
    assert_eq!((1, vec![2]), run(Graph::simple(3, &[(0, 1), (0, 2)])));
    assert_eq!((1, vec![2]), run(Graph::simple(3, &[(0, 1), (1, 2)])));
    assert_eq!((1, vec![0]), run(Graph::simple(3, &[(0, 2), (1, 2)])));
    assert_eq!((2, vec![2]), run(Graph::simple(3, &[(0, 1), (1, 2), (2, 0)])));

    assert_eq!((0, vec![0, 1]), run(Graph::simple(4, &[])));
    assert_eq!((0, vec![3]), run(Graph::simple(4, &[(0, 2)])));
    assert_eq!((0, vec![1]), run(Graph::simple(4, &[(0, 2), (0, 3)])));
    assert_eq!((0, vec![0, 1]), run(Graph::simple(4, &[(0, 1), (2, 3)])));
    assert_eq!((1, vec![3]), run(Graph::simple(4, &[(0, 1), (0, 2), (0, 3)])));
    assert_eq!((1, vec![0, 1]), run(Graph::simple(4, &[(0, 1), (1, 2), (2, 3)])));
    assert_eq!((0, vec![3]), run(Graph::simple(4, &[(0, 1), (0, 2), (0, 2)])));
    assert_eq!((2, vec![0, 1]), run(Graph::simple(4, &[(0, 1), (1, 2), (2, 3), (3, 0)])));
    assert_eq!((1, vec![3]), run(Graph::simple(4, &[(0, 1), (1, 2), (2, 0), (3, 0)])));
    assert_eq!((2, vec![3]), run(Graph::simple(4, &[(0, 1), (1, 2), (2, 0), (3, 0), (3, 1)])));
    assert_eq!((3, vec![2]), run(Graph::simple(4, &[(0, 1), (1, 2), (2, 0), (3, 0), (3, 1), (3, 2)])));
}