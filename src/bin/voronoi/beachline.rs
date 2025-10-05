/// also lifted from the voronoi crate, now using petgraph

use petgraph::{
    graph::{DiGraph, node_index},
};

use crate::Point;
type TripleSite = (Point, Point, Point);

const NIL: usize = !0;

#[derive(Debug)]
pub struct Beachline {
    pub graph: DiGraph<BeachNode, (), usize>,
    pub root: usize,
}

#[derive(Debug)]
pub struct BeachNode {
    pub parent: Option<usize>,
    pub left: Option<usize>,
    pub right: Option<usize>,
    pub item: BeachItem,
}

impl BeachNode {
    pub fn make_root(item: BeachItem) -> Self {
        BeachNode { parent: None, left: None, right: None, item, }
    }

    pub fn make_arc(parent: Option<usize>, item: BeachItem) -> Self {
        if let BeachItem::Arc(_) = item {
            BeachNode { parent, left: None, right: None, item }
        } else {
            panic!("make_arc only accepts arc items!");
        }
    }
}

#[derive(Debug, Clone)]
pub struct Breakpoint { pub left: Point, pub right: Point, pub edge_idx: usize, }
impl Breakpoint {
    fn get_x(&self, yl: f64) -> f64 {
        let ax = self.left.x();
        let bx = self.right.x();
        let ay = self.left.y();
        let by = self.right.y();

        // shift frames
        let bx_s = bx - ax;
        let ay_s = ay - yl;
        let by_s = by - yl;

        let discrim = ay_s * by_s * ((ay_s - by_s) * (ay_s - by_s) + bx_s * bx_s);
        let numer = ay_s * bx_s - discrim.sqrt();
        let denom = ay_s - by_s;

        let mut x_bp = if denom != 0.0 {
            numer / denom
        } else {
            bx_s / 2.
        };
        x_bp += ax; // shift back to original frame

        return x_bp;
    }

    // TODO: handle py == yl case
    pub fn get_y(&self, yl: f64) -> f64 {
        let px = self.left.x();
        let py = self.left.y();

        let bp_x = self.get_x(yl);

        let numer = (px - bp_x) * (px - bp_x);
        let denom = 2. * (py - yl);

        return numer / denom + (py + yl) / 2.;
    }
    
    // see http://www.kmschaal.de/Diplomarbeit_KevinSchaal.pdf, pg 27
    pub fn breakpoints_converge(triple_site: TripleSite) -> bool {
        let (a, b, c) = triple_site;
        let ax = a.x();
        let ay = a.y();
        let bx = b.x();
        let by = b.y();
        let cx = c.x();
        let cy = c.y();

        (ay - by) * (bx - cx) > (by - cy) * (ax - bx)
    }
}

#[derive(Debug, Clone)]
pub struct Arc { pub site: Point, pub site_event: Option<usize>, }

#[derive(Debug, Clone)]
pub enum BeachItem {
    Breakpoint(Breakpoint),
    Arc(Arc)
}

impl Beachline {
    pub fn new() -> Self {
        Beachline {
            graph: petgraph::graph::DiGraph::default(),
            root: NIL,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.graph.edge_count() == 0 && self.graph.node_count() == 0
    }

    pub fn insert_point(&mut self, pt: Point) {
        let arc = Arc { site: pt, site_event: None };
        let item = BeachItem::Arc(arc);
        let node = BeachNode::make_root(item);
        let idx = self.graph.add_node(node);
        self.root = idx.index();
    }

    pub fn get_arc_above(&self, pt: Point) -> usize {
        if self.is_empty() { panic!("can't get_arc_above on an empty beachline!"); }
        let mut current_node = self.root;
        loop {
            let node = &self.graph[node_index(current_node)];
            match node.item {
                BeachItem::Arc(_) => { return current_node; },
                BeachItem::Breakpoint(ref breakpoint) => {
                    let x_bp = breakpoint.get_x(pt.y());
                    if pt.x() < x_bp { current_node = node.left.unwrap() }
                    else { current_node = node.right.unwrap() }
                }
            }
        }
    }

    pub fn tree_minimum(&self, root: usize) -> usize {
        let mut current_node = root;
        while let Some(left) = self.graph[node_index(current_node)].left {
            current_node = left;
        }
        current_node
    }

    pub fn tree_maximum(&self, root: usize) -> usize {
        let mut current_node = root;
        while let Some(right) = self.graph[node_index(current_node)].right {
            current_node = right;
        }
        current_node
    }

    pub fn successor(&self, node: usize) -> Option<usize> {
        if let Some(right) = self.graph[node_index(node)].right {
            return Some(self.tree_minimum(right))
        }
        let mut current_node = Some(node);
        let mut current_parent = self.graph[node_index(node)].parent;
        while current_parent.is_some() && current_node == self.graph[node_index(current_parent.unwrap())].right {
            current_node = current_parent;
            current_parent = self.graph[node_index(current_parent.unwrap())].parent;
        }
        current_parent
    }

    pub fn predecessor(&self, node: usize) -> Option<usize> {
        if let Some(left) = self.graph[node_index(node)].left {
            return Some(self.tree_maximum(left))
        }
        let mut current_node = Some(node);
        let mut current_parent = self.graph[node_index(node)].parent;
        while current_parent.is_some() && current_node == self.graph[node_index(current_parent.unwrap())].left {
            current_node = current_parent;
            current_parent = self.graph[node_index(current_parent.unwrap())].parent;
        }
        current_parent
    }

    pub fn get_left_arc(&self, node: Option<usize>) -> Option<usize> {
        node.and_then(|node| self.predecessor(node)).and_then(|left| self.predecessor(left))
    }

    pub fn get_right_arc(&self, node: Option<usize>) -> Option<usize> {
        node.and_then(|node| self.successor(node)).and_then(|right| self.successor(right))
    }

    pub fn get_leftward_triple(&self, node: usize) -> Option<TripleSite> {
        let l_arc = self.get_left_arc(Some(node));
        let ll_arc = self.get_left_arc(l_arc);
        let this_site = self.get_site(Some(node));
        let l_site = self.get_site(l_arc);
        let ll_site = self.get_site(ll_arc);

        match (this_site, l_site, ll_site) {
            (Some(t), Some(l), Some(ll)) => Some((ll,l,t)),
            _ => None,
        }
    }

    pub fn get_rightward_triple(&self, node: usize) -> Option<TripleSite> {
        let r_arc = self.get_right_arc(Some(node));
        let rr_arc = self.get_right_arc(r_arc);
        let this_site = self.get_site(Some(node));
        let r_site = self.get_site(r_arc);
        let rr_site = self.get_site(rr_arc);

        match (this_site, r_site, rr_site) {
            (Some(t), Some(r), Some(rr)) => Some((t,r,rr)),
            _ => None,
        }
    }

    pub fn get_centered_triple(&self, node: usize) -> Option<TripleSite> {
        let r_arc = self.get_right_arc(Some(node));
        let l_arc = self.get_left_arc(Some(node));
        let this_site = self.get_site(Some(node));
        let r_site = self.get_site(r_arc);
        let l_site = self.get_site(l_arc);

        match (this_site, l_site, r_site) {
            (Some(t), Some(l), Some(r)) => Some((l,t,r)),
            _ => None,
        }
    }

    pub fn set_right_site(&mut self, node: usize, site: Point) {
        if let BeachItem::Breakpoint(ref mut bp) = self.graph[node_index(node)].item {
            bp.right = site;
        } else {
            panic!("set_right_site can't handle anything other than breakpoints!");
        }
    }

    pub fn set_left_site(&mut self, node: usize, site: Point) {
        if let BeachItem::Breakpoint(ref mut bp) = self.graph[node_index(node)].item {
            bp.left = site;
        } else {
            panic!("set_left_site can't handle anything other than breakpoints!");
        }
    }

    pub fn get_site(&self, node: Option<usize>) -> Option<Point> {
        node.and_then(|node| match self.graph[node_index(node)].item {
            BeachItem::Arc(ref arc) => Some(arc.site),
            _ => None,
        })
    }

    pub fn get_edge(&self, node: usize) -> usize {
        if let BeachItem::Breakpoint(ref bp) = self.graph[node_index(node)].item {
            bp.edge_idx
        } else {
            panic!("get_edge can't handle anything other than breakpoints!");
        }
    }
}

