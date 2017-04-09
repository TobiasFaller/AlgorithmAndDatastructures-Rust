extern crate zip;

use self::zip::ZipArchive;
use self::zip::result::ZipError;

use std::collections::HashSet;
use std::error::Error as StdError;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::fs::File;
use std::io::{BufRead, BufReader, Error as IOError};
use std::num::{ParseFloatError, ParseIntError};
use std::result::Result;
use std::string::String;
use std::vec::Vec;

#[derive(Debug)]
pub enum Error {
	FormatError { message: String },
	IoError(IOError),
	ParseIntError(ParseIntError),
	ParseFloatError(ParseFloatError),
	ZipError(ZipError)
}

impl From<ParseIntError> for Error {
    fn from(err: ParseIntError) -> Error {
        Error::ParseIntError(err)
    }
}

impl From<ParseFloatError> for Error {
    fn from(err: ParseFloatError) -> Error {
        Error::ParseFloatError(err)
    }
}

impl From<IOError> for Error {
    fn from(err: IOError) -> Error {
        Error::IoError(err)
    }
}

impl From<zip::result::ZipError> for Error {
    fn from(err: ZipError) -> Error {
        Error::ZipError(err)
    }
}

impl<'a> From<&'a str> for Error {
	fn from(err: &str) -> Error {
		Error::FormatError {
			message: err.to_owned()
		}
	}
}

impl From<String> for Error {
	fn from(err: String) -> Error {
		Error::FormatError {
			message: err
		}
	}
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter) -> FmtResult {
		match *self {
			Error::IoError(ref err) => write!(f, "IO Error: {}", err),
			Error::ParseIntError(ref err) => write!(f, "Parse Error: {}", err),
			Error::ParseFloatError(ref err) => write!(f, "Parse Error: {}", err),
			Error::ZipError(ref err) => write!(f, "Zip Error: {}", err),
			Error::FormatError { ref message } => write!(f, "Format error: {}", message)
		}
	}
}

impl StdError for Error {
	fn description(&self) -> &str {
		match *self {
			Error::IoError(ref err) => err.description(),
			Error::ParseIntError(ref err) => err.description(),
			Error::ParseFloatError(ref err) => err.description(),
			Error::ZipError(ref err) => err.description(),
			Error::FormatError { ref message } => &message
		}
	}
	
	fn cause(&self) -> Option<&StdError> {
		match *self {
			Error::IoError(ref err) => Some(err),
			Error::ParseIntError(ref err) => Some(err),
			Error::ParseFloatError(ref err) => Some(err),
			Error::ZipError(ref err) => Some(err),
			Error::FormatError { .. } => None
		}
	}
}

struct Node<'a> {
	id: usize,
	latitude: f64,
	longitude: f64,
	traceback_arc: Option<&'a Arc>,
	settled: bool,
	distance: Option<u64>
}

struct Arc {
	head_node_id: usize,
	tail_node_id: usize,
	distance: u64,
	max_speed: u64,
	costs: u64
}

pub struct Graph<'a> {
	nodes: Box<Vec<Node<'a>>>,
	adjacency_lists: Box<Vec<Vec<Arc>>>
}

#[allow(dead_code)]
impl<'a> Graph<'a> {
	
	fn read_lines<R: BufRead>(&mut self, buf: R) -> Result<(), Error> {
		let mut line_number = 0;
		let mut total_line_number = 0;
		
		let mut node_count = 0;
		let mut arc_count = 0;
		
		for line_res in buf.lines() {
			let line = try!(line_res);
			let line = line.trim();
			
			total_line_number += 1;
			
			if line.starts_with("#") || line.is_empty() {
				continue;
			}
			
			line_number += 1;
			
			let parts: Vec<&str> = line.split(' ').collect();
			if parts.len() < 1 {
				return Err(Error::from(format!("Invalid graph file! (general, line {})", total_line_number)));
			}
			
			match line_number {
				1 => {
					node_count = try!(parts[0].parse::<usize>());
					self.nodes.reserve_exact(node_count);
				},
				2 => {
					arc_count = try!(parts[0].parse::<usize>());
					self.adjacency_lists.reserve_exact(arc_count);
					
					for _ in 0..arc_count {
						self.adjacency_lists.push(Vec::new());
					}
				},
				_ => {
					if line_number < node_count + 3 {
						if parts.len() != 3 {
							return Err(Error::from(format!("Invalid graph file! (Invalid node, line {})", total_line_number)));
						}
						
						let node_id = try!(parts[0].parse::<usize>());
						let latitude = try!(parts[1].parse::<f64>());
						let longitude = try!(parts[2].parse::<f64>());
						
						self.nodes.push(Node {
							id: node_id,
							latitude: latitude,
							longitude: longitude,
							traceback_arc: None,
							settled: false,
							distance: None
						});
 					} else if line_number < node_count + arc_count + 3 {
						if parts.len() != 4 {
							return Err(Error::from(format!("Invalid graph file! (Invalid arc, line {})", total_line_number)));
						}
						
						let tail_node = try!(parts[0].parse::<usize>());
						let head_node = try!(parts[1].parse::<usize>());
						
						let distance = try!(parts[2].parse::<u64>());
						let max_speed = try!(parts[3].parse::<u64>());
						
						self.adjacency_lists[tail_node].push(Arc {
							tail_node_id: tail_node,
							head_node_id: head_node,
							distance: distance,
							max_speed: max_speed,
							costs: distance
						});
						
						// We create an undirected graph
						self.adjacency_lists[head_node].push(Arc {
							tail_node_id: head_node,
							head_node_id: tail_node,
							distance: distance,
							max_speed: max_speed,
							costs: distance
						});
 					} else {
	 					return Err(Error::from(format!("Invalid graph file! (Additional lines, line {})", total_line_number)));
 					}
				}
			}
		}
		
		Ok(())
	}
	
	pub fn read_graph_from_file(name: &str) -> Result<Graph, Error> {
		let file = try!(File::open(name));
		let mut archive = try!(ZipArchive::new(file));
		
		let mut graph = Graph {
			nodes: box Vec::new(),
			adjacency_lists: box Vec::new()
		};
		
		for index in 0 .. archive.len() {
			let entry = try!(archive.by_index(index));
			let buf = BufReader::new(entry);
			try!(graph.read_lines(buf));
		}
		
		Ok(graph)
	}
	
	fn set_arc_costs_to_travel_time(&mut self, max_vehicle_speed: u64) {
		for arcs in self.adjacency_lists.iter_mut() {
			for arc in arcs.iter_mut() {
				let mut max_speed = arc.max_speed;
				if max_vehicle_speed < max_speed {
					max_speed = max_vehicle_speed;
				}
				
				arc.costs = ((arc.distance as f64) * 3.6 / (max_speed as f64)) as u64;
			}
		}
	}
	
	fn set_arc_costs_to_distance(&mut self) {
		for arcs in self.adjacency_lists.iter_mut() {
			for arc in arcs.iter_mut() {
				arc.costs = arc.distance;
			}
		}
	}
	
	pub fn num_nodes(&self) -> usize {
		self.nodes.len()
	}
	
	pub fn num_arcs(&self) -> usize {
		self.adjacency_lists.len()
	}
	
	fn compute_reachable_nodes(&self, node_id: usize) -> (usize, Box<Vec<u8>>) {
		let mut marked_nodes = box vec![0_u8; self.num_nodes()];
		let mut num_marked = 1;
		
		let mut pending_nodes = box HashSet::<usize>::new();
		pending_nodes.insert(node_id);
		
		while !pending_nodes.is_empty() {
			let mut next_nodes = box HashSet::<usize>::new();
			
			for node in pending_nodes.drain() {
				if marked_nodes[node] == 1 {
					continue;
				}
				
				marked_nodes[node] = 1;
				num_marked += 1;
				
				for arc in self.adjacency_lists[node].iter() {
					if marked_nodes[arc.head_node_id] == 0 {
						next_nodes.insert(arc.head_node_id);
					}
				}
			}
			
			pending_nodes = next_nodes;
		}
		
		return (num_marked, marked_nodes);
	}
	
	pub fn compute_lcc(&self) -> (usize, Box<Vec<usize>>){
		let node_count = self.num_nodes();
		
		let mut unvisited_nodes = box vec![0_u8; node_count];
		let mut marked_nodes = box Vec::<usize>::new();
		let mut lcc = (0, box Vec::<usize>::new());
		
		for i in 0..node_count {
			if unvisited_nodes[i] == 1 {
				continue;
			}
			
			let (num_marked, reachable_nodes) = self.compute_reachable_nodes(i);
			if num_marked == 0 {
				continue;
			}
			
			marked_nodes.clear();
			for j in 0..node_count {
				if reachable_nodes[j] == 0 {
					continue;
				}
				if j > i {
					unvisited_nodes[j] = 1;
				}

				if num_marked > lcc.0 {
					marked_nodes.push(i);
				}
			}
			
			if num_marked > lcc.0 {
				lcc = (num_marked, marked_nodes.clone());
			}
		}
		
		return lcc;
	}
}
