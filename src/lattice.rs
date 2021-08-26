pub mod node;

use std::i32;

use self::node::Node;
use crate::dic::grammar::Grammar;
use crate::prelude::*;

/// Lattice for tokenization
pub struct Lattice<'a> {
    grammar: &'a Grammar<'a>,
    /// The byte_length of target text
    size: usize,
    /// List of lattice nodes which ends at the byte_idx
    pub end_lists: Vec<Vec<Node>>,
}

impl<'a> Lattice<'a> {
    /// Creates a lattice with bos node node
    pub fn new(grammar: &'a Grammar, size: usize) -> Lattice<'a> {
        let mut end_lists = vec![Vec::<Node>::new(); size + 1];

        let bos_node = Node::new_bos();
        end_lists[0].push(bos_node);

        Lattice {
            grammar,
            size,
            end_lists,
        }
    }

    /// Inserts a node to the given range
    pub fn insert(&mut self, begin: usize, end: usize, mut node: Node) -> SudachiResult<()> {
        node.set_range(begin, end);
        self.connect_node(&mut node)?;
        self.end_lists[end].push(node);

        Ok(())
    }

    /// Connect node to previous nodes and calculate total_cost if possible
    fn connect_node(&self, r_node: &mut Node) -> SudachiResult<()> {
        let begin = r_node.begin;
        r_node.total_cost = i32::MAX;

        for (i, l_node) in self.end_lists[begin].iter().enumerate() {
            if !l_node.is_connected_to_bos {
                continue;
            }

            let connect_cost = self
                .grammar
                .get_connect_cost(l_node.right_id, r_node.left_id)?;
            let cost = l_node.total_cost + connect_cost as i32;
            if cost < r_node.total_cost {
                r_node.total_cost = cost;
                r_node.best_previous_node_index = Some((begin, i));
            }
        }
        r_node.total_cost += r_node.cost as i32;

        r_node.is_connected_to_bos = r_node.best_previous_node_index.is_some();

        Ok(())
    }

    /// Inserts eos node
    pub fn connect_eos_node(&mut self) -> SudachiResult<()> {
        let eos_node = Node::new_eos(self.size);
        self.insert(eos_node.begin, eos_node.end, eos_node)
    }

    /// Returns if the lattice has node which ends at the given byte_idx
    pub fn has_previous_node(&self, index: usize) -> bool {
        !self.end_lists[index].is_empty()
    }

    /// Calculate and Returns the best path from lattice
    pub fn get_best_path(&self) -> SudachiResult<Vec<Node>> {
        // TODO: reference of eos_node in struct `Lattice` ?
        let eos_node = self.end_lists[self.size]
            .last()
            .ok_or(SudachiError::MissingLaticePath)?;

        if !eos_node.is_connected_to_bos {
            return Err(SudachiError::EosBosDisconnect);
        }

        let mut path = Vec::new();
        let mut node = eos_node;
        loop {
            path.push(node.clone());

            let (i, j) = node
                .best_previous_node_index
                .ok_or(SudachiError::MissingLaticePath)?;

            if (i, j) == (0, 0) {
                break;
            }
            node = &self.end_lists[i][j];
        }
        path.reverse();
        path.pop(); // remove EOS

        Ok(path)
    }

    /// Dumps lattice
    pub fn dump(&self, grammar: &Grammar) -> SudachiResult<()> {
        let mut i = 0;
        for r_nodes in self.end_lists.iter().rev() {
            for r_node in r_nodes {
                print!("{}: {}: ", i, r_node);
                for l_node in &self.end_lists[r_node.begin] {
                    let connect_cost = grammar.get_connect_cost(l_node.right_id, r_node.left_id)?;
                    let cost = l_node.total_cost + connect_cost as i32;
                    print!("{} ", cost);
                }
                println!();
                i += 1;
            }
        }
        Ok(())
    }
}
