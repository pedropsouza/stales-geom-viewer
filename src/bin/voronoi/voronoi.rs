use macroquad::prelude::*;
use genmap::GenMap;

pub mod event;
pub mod beachline;
pub mod dcel;

use event::*;
use beachline::{BeachItem, Beachline, Breakpoint};
use petgraph::{graph::node_index, visit::EdgeRef};
use stales_geom_viewer::point::Point;

use std::{
    cmp::{Ord, Ordering}, collections::HashMap, default::Default, io::Write, iter::Iterator, time::Instant
};

use stales_geom_viewer::{
    utils,
    common_traits::*,
    geom::{self, *, Vertex},
};
use euclid::default::Vector2D;

type Color = macroquad::color::Color;

enum Object {
    Point(geom::Vertex),
    CircleObj(geom::Circle),
    LineObj(geom::Line2D),
    PolyObj(geom::Polygon),
}

impl Draw for Object {
    fn draw(&self) {
        match self {
            Object::Point(p) => p.draw(),
            Object::CircleObj(c) => c.draw(),
            Object::LineObj(l) => l.draw(),
            Object::PolyObj(p) => p.draw(),
        }
    }
    fn vertices(&self) -> Vec<geom::Vertex> {
        match self {
            Object::Point(p) => p.vertices(),
            Object::CircleObj(c) => c.vertices(),
            Object::LineObj(l) => l.vertices(),
            Object::PolyObj(p) => p.vertices(),
        }
    }
}

struct State {
    pub objects: GenMap<Object>,
    pub clear_color: Color,
}

impl Default for State {
    fn default() -> Self {
        Self {
            objects: GenMap::with_capacity(1000),
            clear_color: BLACK,
        }
    }
}

impl State {
    fn add_line(&mut self, l: geom::Line2D) -> genmap::Handle {
        self.objects.insert(Object::LineObj(l))

    }
    fn add_circle(&mut self, c: geom::Circle) -> genmap::Handle {
        self.objects.insert(Object::CircleObj(c))
    }
    fn add_poly(&mut self, p: geom::Polygon) -> genmap::Handle {
        self.objects.insert(Object::PolyObj(p))
    }
    fn all_elements(&self) -> impl Iterator<Item = (genmap::Handle, &dyn Element)> {
        self.objects.iter().flat_map(|x| {
            self.objects.get(x).and_then(|obj|
                match obj {
                    Object::Point(_p) => None,
                    Object::CircleObj(c) => Some(c as &dyn Element),
                    Object::LineObj(l) => Some(l as &dyn Element),
                    Object::PolyObj(_p) => None,
                }).map(|elem| (x, elem))
            })
    }
    fn text_digest(&self) -> String {
        let line_cnt = self.objects
                           .iter().flat_map(|x| self.objects.get(x))
                           .filter(|x| matches!(x, Object::LineObj(_))).count();
        let circle_cnt = self.objects
                             .iter().flat_map(|x| self.objects.get(x))
                             .filter(|x| matches!(x, Object::CircleObj(_))).count();
        let vertex_cnt = self.objects
                             .iter().flat_map(|x| self.objects.get(x))
                                    .map(Draw::vertices).fold(0usize, |acc,verts| acc + verts.len());
        let frametime = get_frame_time();
        format!(r"
num. of lines: {line_cnt}
num. of circles: {circle_cnt}
num. of vertices: {vertex_cnt}
frametime: {frametime}
")
    }
}

#[derive(Debug)]
enum LogTag {
    Mouse, FrameTime, Select,
}

#[derive(PartialEq, Eq, Ord, Debug)]
pub struct EventPrio<T: Ord + Eq> {
    pub x: T,
    pub y: T,
}

impl<T: Ord + Eq> PartialOrd for EventPrio<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.y.partial_cmp(&other.y) {
            Some(Ordering::Equal) => {}
            ord => return ord,
        }
        self.x.partial_cmp(&other.x)
    }
}
#[derive(Debug)]
pub struct Algo {
    pub event_queue: EventQueue,
    pub beachline: Beachline,
    pub output: dcel::DCEL,
}

impl Algo {
    pub fn new(points: &Vec<Point>) -> Self {
        let mut queue = EventQueue::new();
        for point in points {
                queue.push(Event::Site(*point));
        }
        let beachline = Beachline::new();
        Self {
            event_queue: queue,
            beachline,
            output: dcel::DCEL::new(),
        }
    }

    // returns true if it still has work to do
    pub fn process_next_event(&mut self) -> bool {
        if let Some(event) = self.event_queue.pop() {
            trace!("processing event {:?}", event);
            match event {
                Event::Site(point) => {
                    self.handle_site_event(point);
                }
                Event::Circle(data) => {
                    self.handle_circle_event(data);
                }
            }
            return true
        }
        false
    }

    pub fn handle_site_event(&mut self, site: Point) {
        trace!("processing site {:?}", site);

        if self.beachline.is_empty() {
            trace!("Beachline empty, inserting point.");
            self.beachline.insert_point(site);
            return;
        }

        let arc_above = self.beachline.get_arc_above(site);

        // remove false alarm circle event
        self.remove_circle_event(arc_above);

        let new_node = self.split_arc(arc_above, site);
        if let Some(left_triple) = self.beachline.get_leftward_triple(new_node) {
            trace!("Checking leftward triple {:?}", left_triple);
            if Breakpoint::breakpoints_converge(left_triple) {
                trace!("Found converging triple");
                let arc = self.beachline.get_left_arc(Some(new_node)).unwrap();
                self.make_circle_event(arc, left_triple);
            }
        }
        if let Some(right_triple) = self.beachline.get_rightward_triple(new_node) {
            trace!("Checking rightward triple {:?}", right_triple);
            if Breakpoint::breakpoints_converge(right_triple) {
                trace!("Found converging triple");
                let arc = self.beachline.get_right_arc(Some(new_node)).unwrap();
                self.make_circle_event(arc, right_triple);
            }
        }
    }
    // return: indices of predecessor, successor, parent, 'other'
    // where 'other' is the one of predecessor or sucessor that
    // is not the parent of the leaf.
    fn delete_leaf(&mut self, leaf: usize) -> (usize, usize, usize, usize) {
        let beachline = &mut self.beachline;
        let pred = beachline.predecessor(leaf).unwrap();
        let succ = beachline.successor(leaf).unwrap();
        let parent = beachline.graph[node_index(leaf)].parent.unwrap();
        let grandparent = beachline.graph[node_index(parent)].parent.unwrap();

        let other = if parent == pred { succ } else { pred };

        // find sibling
        let sibling;
        if beachline.graph[node_index(parent)].right.unwrap() == leaf {
            sibling = beachline.graph[node_index(parent)].left.unwrap();
        } else if beachline.graph[node_index(parent)].left.unwrap() == leaf {
            sibling = beachline.graph[node_index(parent)].right.unwrap();
        } else {
            panic!("family strife! parent does not acknowledge leaf!");
        }

        // transplant the sibling to replace the parent
        beachline.graph[node_index(sibling)].parent = Some(grandparent);
        if beachline.graph[node_index(grandparent)].left.unwrap() == parent {
            beachline.graph[node_index(grandparent)].left = Some(sibling);
        } else if beachline.graph[node_index(grandparent)].right.unwrap() == parent {
            beachline.graph[node_index(grandparent)].right = Some(sibling);
        } else {
            panic!("family strife! grandparent does not acknowledge parent!");
        }

        // correct the site on 'other'
        if other == pred {
            let new_other_succ = beachline.successor(other).unwrap();
            let new_site = beachline.get_site(Some(new_other_succ)).unwrap();
            beachline.set_right_site(other, new_site);
        } else {
            let new_other_pred = beachline.predecessor(other).unwrap();
            let new_site = beachline.get_site(Some(new_other_pred)).unwrap();
            beachline.set_left_site(other, new_site);
        }

        (pred, succ, parent, other)
    }

    fn handle_circle_event(&mut self, data: CircleEvent) {
        let leaf = data.vanishing_arc;
        let left_neighbor = self.beachline.get_left_arc(Some(leaf)).unwrap();
        let right_neighbor = self.beachline.get_right_arc(Some(leaf)).unwrap();
        let (pred, succ, parent, other) = self.delete_leaf(leaf);

        // removing site events involving disappearing arc
        self.remove_circle_event(leaf);
        self.remove_circle_event(left_neighbor);
        self.remove_circle_event(right_neighbor);

        let (twin1, twin2) = self.output.add_twins();

        // make a vertex at the circle center
        let center_vertex = dcel::Vertex { coordinates: data.center, incident_edge: twin1, alive: true};
        let center_vertex_ind = self.output.vertices.len();
        self.output.vertices.push(center_vertex);

        // hook up next pointers on halfedges
        let pred_edge = self.beachline.get_edge(pred);
        let succ_edge = self.beachline.get_edge(succ);
        let parent_edge = self.beachline.get_edge(parent);
        let other_edge = self.beachline.get_edge(other);

        let pred_edge_twin = self.output.halfedges[pred_edge].twin;
        let succ_edge_twin = self.output.halfedges[succ_edge].twin;

        self.output.halfedges[parent_edge].origin = center_vertex_ind;
        self.output.halfedges[other_edge].origin = center_vertex_ind;
        self.output.halfedges[twin1].origin = center_vertex_ind;

        self.output.halfedges[pred_edge_twin].next = succ_edge;
        self.output.halfedges[succ_edge_twin].next = twin1;
        self.output.halfedges[twin2].next = pred_edge;

        if let BeachItem::Breakpoint(ref mut breakpoint) = self.beachline.graph[node_index(other)].item {
            breakpoint.edge_idx = twin2;
        }

        if let Some(left_triple) = self.beachline.get_centered_triple(left_neighbor) {
            trace!("Checking leftward triple {:?}, {:?}, {:?}", left_triple.0, left_triple.1, left_triple.2);
            if Breakpoint::breakpoints_converge(left_triple) {
                trace!("Found converging triple");
                self.make_circle_event(left_neighbor, left_triple);
            }
        }
        if let Some(right_triple) = self.beachline.get_centered_triple(right_neighbor) {
            trace!("Checking rightward triple {:?}, {:?}, {:?}", right_triple.0, right_triple.1, right_triple.2);
            if Breakpoint::breakpoints_converge(right_triple) {
                trace!("Found converging triple");
                self.make_circle_event(right_neighbor, right_triple);
            }
        }
    }

    fn remove_circle_event(&mut self, arc_idx: usize) {
        let mut circle_event = None;
        if let BeachItem::Arc(ref mut arc) = self.beachline.graph[node_index(arc_idx)].item {
            circle_event = arc.site_event;
            arc.site_event = None;
        }

        if let Some(ev) = circle_event {
            self.event_queue.remove(ev);
        }
    }

    #[allow(non_snake_case)]
    // return: the index of the node for the new arc
    fn split_arc(&mut self, arc: usize, site: Point) -> usize {
        use beachline::*;
        trace!("splitting arc {:?}", arc);
        let parent = self.beachline.graph[node_index(arc)].parent;

        let mut arc_pt = Point::new(0.0, 0.0);
        if let BeachItem::Arc(ref this_arc) = self.beachline.graph[node_index(arc)].item {
            arc_pt = this_arc.site;
        }

        let (twin1, twin2) = self.output.add_twins();

        let breakpoint_AB = Breakpoint { left: arc_pt, right: site, edge_idx: twin1 };
        let breakpoint_BA = Breakpoint { left: site, right: arc_pt, edge_idx: twin2 };

        let internal_AB = BeachItem::Breakpoint(breakpoint_AB);
        let internal_BA = BeachItem::Breakpoint(breakpoint_BA);

        let arc_A1 = beachline::Arc { site: arc_pt, site_event: None };
        let arc_A2 = beachline::Arc { site: arc_pt, site_event: None };
        let arc_B  = beachline::Arc { site, site_event: None };

        let leaf_A1 = BeachItem::Arc(arc_A1);
        let leaf_A2 = BeachItem::Arc(arc_A2);
        let leaf_B = BeachItem::Arc(arc_B);

        let ind_AB = self.beachline.graph.node_count(); // is this sequential invariant upheld?
        let ind_BA = ind_AB + 1;
        let ind_A1 = ind_AB + 2;
        let ind_B  = ind_AB + 3;
        let ind_A2 = ind_AB + 4;

        let node_AB = BeachNode { parent: parent, left: Some(ind_A1), right: Some(ind_BA), item: internal_AB};
        self.beachline.graph.add_node(node_AB);
        let node_BA = BeachNode { parent: Some(ind_AB), left: Some(ind_B), right: Some(ind_A2), item: internal_BA};
        self.beachline.graph.add_node(node_BA);
        let node_A1 = BeachNode::make_arc(Some(ind_AB), leaf_A1);
        self.beachline.graph.add_node(node_A1);
        let node_B = BeachNode::make_arc(Some(ind_BA), leaf_B);
        self.beachline.graph.add_node(node_B);
        let node_A2 = BeachNode::make_arc(Some(ind_BA), leaf_A2);
        self.beachline.graph.add_node(node_A2);

        let mut add_edge = |a,b| { self.beachline.graph.add_edge(node_index(a), node_index(b), ()); };
        add_edge(ind_AB, ind_A1);
        add_edge(ind_AB, ind_BA);
        add_edge(ind_BA, ind_B);
        add_edge(ind_BA, ind_A2);
        
        if let Some(parent_ind) = parent {
            for edge in self.beachline.graph.edges_connecting(node_index(parent_ind), node_index(arc))
                                            .map(|edge| edge.id()).collect::<Vec<_>>() {
                self.beachline.graph.remove_edge(edge);
            }
            self.beachline.graph.add_edge(node_index(parent_ind), node_index(ind_AB), ());
            let parent_node = &mut self.beachline.graph[node_index(parent_ind)];
            if parent_node.right.is_some() && parent_node.right.unwrap() == arc {
                parent_node.right = Some(ind_AB);
            } else if parent_node.left.is_some() && parent_node.left.unwrap() == arc {
                parent_node.left = Some(ind_AB);
            } else {
                panic!("tree is borked");
            }
        } else {
            self.beachline.root = Some(ind_AB);
        }
        return ind_B;
    }

    fn make_circle_event(&mut self, arc: usize, triple: (Point, Point, Point)) {
        if let Some(circle_center) = circle_center(triple) {
            let circle_bottom = circle_bottom(triple).unwrap();
            let this_event = Event::Circle(CircleEvent {
                center: circle_center,
                radius: circle_bottom.0 - circle_center.y(),
                vanishing_arc: arc,
                id: 0,
            });
            if let BeachItem::Arc(ref mut arc) = self.beachline.graph[node_index(arc)].item {
                arc.site_event = Some(self.event_queue.push(this_event));
            }
        }
    }
}

#[macroquad::main("Voronoi")]
async fn main() {
    const WIDTH: f32 = 1800.0;
    const HEIGHT: f32 = 1000.0;
    request_new_screen_size(WIDTH, HEIGHT);
    let mut state: State = Default::default();
    let startup = Instant::now();
    let mut prev_mouse_pos = mouse_position();

    stderrlog::new()
        .module(module_path!())
        .verbosity(log::LevelFilter::Trace)
        .init().unwrap();
    log::log!(log::Level::Error, "aaaaarhg");
    log::info!("up and running");
    
    let mut logfile = std::fs::File::create("./log.txt").expect("can't create \"./log.txt\" log file!");
    const CIRCLE_RADIUS: f32 = 4.0;

    let bounds = (0.0..WIDTH, 0.0..HEIGHT);
    let random_float_points = utils::random_points(1000, bounds.clone());

    for p in &random_float_points {
        state.add_circle(geom::Circle {
            center: Vertex::new(p.x, p.y,
                                Some(utils::random_color())),
            radius: CIRCLE_RADIUS,
        });
    }

    let mut voronoi_state = Algo::new(&vec![]);
    let mut voronoi_calc = |state: &State| {
        let mut input_verts = state.all_elements().map(|(_,elem)| {
            let center = elem.compute_aabb().center();
            Point::new(center.x as f64, center.y as f64)
        }).collect();
        voronoi_state = Algo::new(&input_verts);
        while voronoi_state.process_next_event() {};

        let mut interim_dcel = voronoi_state.output.clone();
        add_bounding_box(WIDTH.max(HEIGHT).into(), &voronoi_state.beachline, &mut interim_dcel);
        dcel::add_faces(&mut interim_dcel);

        let poly = dcel_to_wire_poly(&interim_dcel);

        // let delauney = {
        //     // all the vertices are the same as the input to the voronoi algo
        //     // we only need the voronoi result to know which edges to create
        //     let mut poly = Polygon {
        //         verts: input_verts.iter().map(|v| Vertex::new(v.x() as f32, v.y() as f32, Some(utils::random_color()))).collect(),
        //         ..Default::default()
        //     };
        //     //for 
        // }
        poly
    };

    let mut voronoi_poly = voronoi_calc(&state);

    loop {
        let tick_time = {
            let t = Instant::now().duration_since(startup);
            (t.as_secs(), t.subsec_nanos())
        };

        let mut log_line = |tag: LogTag, msg: &str| {
            writeln!(&mut logfile, "({tag:?}) [{}s{}ns]: {msg}", tick_time.0, tick_time.1).expect("couldn't write log line")
        };

        log_line(LogTag::FrameTime, &format!("{} / {}", get_frame_time(), get_fps()));

        if is_quit_requested() { break }
        clear_background(state.clear_color);

        for handle in state.objects.iter() {
            let object = state.objects.get(handle).unwrap();
            object.draw();
        }

        voronoi_poly.draw();

        { // Mouse handling
            let mouse_pos = mouse_position();
            if mouse_pos != prev_mouse_pos {
                log_line(LogTag::Mouse, &format!("pos {},{}", mouse_pos.0, mouse_pos.1));
                prev_mouse_pos = mouse_pos;
            }

            if is_mouse_button_pressed(MouseButton::Left) || is_mouse_button_pressed(MouseButton::Right) {
                log_line(LogTag::Mouse, &format!("clicked {},{}", mouse_pos.0, mouse_pos.1));

                let mut hit_elem = None;
                let delete = is_mouse_button_pressed(MouseButton::Right);
                for (handle,element) in state.all_elements() {
                    if element.contains_point(&Vector2D::new(mouse_pos.0, mouse_pos.1)) {
                        log_line(LogTag::Select, &format!("selected {:?}", element));
                        hit_elem = Some(handle);
                        break;
                    }
                }
                if let Some(elem) = hit_elem {
                    if delete {
                        state.objects.remove(elem);
                    }
                } else {
                    state.add_circle(geom::Circle {
                        center: Vertex::new(mouse_pos.0, mouse_pos.1, Some(utils::random_color())),
                        radius: CIRCLE_RADIUS,
                    });
                }

                voronoi_poly = voronoi_calc(&state);
            }
        }

        draw_text("IT WORKS!", 20.0, 20.0, 30.0, DARKGRAY);

        if is_key_released(KeyCode::R) {
            println!("{}", state.text_digest())
        }
        next_frame().await
    }
}

pub fn dcel_to_wire_poly(source: &dcel::DCEL) -> Polygon {
    let mut poly = Polygon::default();
    let mut verts_map = HashMap::new();
    for face in make_polygons(&source) {
        let mut vert_ids = vec![];
        for vert in face.iter() {
            let idx = verts_map
                .entry(Point::new(vert.x(), vert.y()))
                .or_insert_with_key(|vert| {
                    poly.verts.push(
                        Vertex::new(
                            vert.x() as f32, vert.y() as f32,
                            Some(utils::random_color())
                        )
                    );
                    poly.verts.len()-1
                });
            vert_ids.push(*idx);
        }
        for (a,b) in vert_ids.iter()
                             .take(vert_ids.len()-1)
                             .zip(vert_ids.iter().skip(1).cycle()) {
            poly.edges.push((*a, *b, utils::random_color()));
        }
    }
    poly
}

// impl for our DCEL newtype
pub fn make_polygons(dcel: &dcel::DCEL) -> Vec<Vec<Point>> {
    let mut result = vec![];
    for face in &dcel.faces {
        if !face.alive { continue; }
        let mut this_poly = vec![];
        let start_edge = face.outer_component;
        let mut current_edge = start_edge;
        loop {
            this_poly.push(dcel.get_origin(current_edge));
            current_edge = dcel.halfedges[current_edge].next;
            if current_edge == start_edge { break; }
        }
        result.push(this_poly);
    }

    // remove the outer face
    result.sort_by(|a, b| a.len().cmp(&b.len()));
    result.pop();

    return result;
}

/// Constructs the line segments of the Voronoi diagram.
pub fn make_line_segments(dcel: &dcel::DCEL) -> Vec<Segment> {
    const NIL: usize = !0;
    let mut result = vec![];
    for halfedge in &dcel.halfedges {
        if halfedge.origin != NIL && halfedge.next != NIL && halfedge.alive {
            if dcel.halfedges[halfedge.next].origin != NIL {
                result.push([dcel.vertices[halfedge.origin].coordinates,
                    dcel.get_origin(halfedge.next)])
            }
        }
    }
    result
}

fn outside_bb(pt: Point, box_size: f64) -> bool {
    let delta = 0.1;
    pt.x() < 0. - delta || pt.x() > box_size + delta || pt.y() < 0. - delta || pt.y() > box_size + delta
}

fn add_bounding_box(boxsize: f64, beachline: &beachline::Beachline, dcel: &mut dcel::DCEL) {
    extend_edges(beachline, dcel);

    let delta = 50.;
    let bb_top =    [Point::new(0. - delta, 0.),         Point::new(boxsize + delta, 0.)];
    let bb_bottom = [Point::new(0. - delta, boxsize),    Point::new(boxsize + delta, boxsize)];
    let bb_left =   [Point::new(0.,         0. - delta), Point::new(0.,              boxsize + delta)];
    let bb_right =  [Point::new(boxsize,    0. - delta), Point::new(boxsize,         boxsize + delta)];

    dcel::add_line(bb_top, dcel);
    dcel::add_line(bb_right, dcel);
    dcel::add_line(bb_left, dcel);
    dcel::add_line(bb_bottom, dcel);

    dcel.set_prev();

    for vert in 0..dcel.vertices.len() {
        let this_pt = dcel.vertices[vert].coordinates;
        if outside_bb(this_pt, boxsize) {
            dcel.remove_vertex(vert);
        }
    }

}

// This just extends the edges past the end of the bounding box
fn extend_edges(beachline: &Beachline, dcel: &mut dcel::DCEL) {
    if beachline.root.is_none() { return; }
    let mut current_node = beachline.tree_minimum(beachline.root.unwrap());
    trace!("\n\n");
    loop {
        match beachline.graph[node_index(current_node)].item {
            BeachItem::Arc(_) => {},
            BeachItem::Breakpoint(ref breakpoint) => {
                let this_edge = breakpoint.edge_idx;
                trace!("Extending halfedge {:?} with breakpoint {:?}, {:?}", this_edge, breakpoint.left, breakpoint.right);
                let this_x = breakpoint.get_x(-1000.0);
                let this_y = breakpoint.get_y(-1000.0);

                let vert = dcel::Vertex {coordinates: Point::new(this_x, this_y), incident_edge: this_edge, alive: true};
                let vert_ind = dcel.vertices.len();

                dcel.halfedges[this_edge].origin = vert_ind;
                let this_twin = dcel.halfedges[this_edge].twin;
                dcel.halfedges[this_twin].next = this_edge;

                dcel.vertices.push(vert);
            }
        }
        if let Some(next_node) = beachline.successor(current_node) {
            current_node = next_node;
        } else { break; }
    }

}
