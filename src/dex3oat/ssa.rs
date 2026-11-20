use std::collections::{HashMap, HashSet};

use crate::dex3oat::graph::{BlockId, Graph};

// cite: art nodes.cc#ComputeDominanceInformation
pub fn reverse_post_order(graph: &Graph) -> Vec<BlockId> {
    let n = graph.blocks.len();
    let mut visited = vec![false; n];
    let mut post = Vec::new();
    let mut stack: Vec<(BlockId, usize)> = Vec::new();
    if graph.entry >= n {
        return Vec::new();
    }
    visited[graph.entry] = true;
    stack.push((graph.entry, 0));
    while let Some(&(block, index)) = stack.last() {
        if index < graph.blocks[block].successors.len() {
            stack.last_mut().unwrap().1 += 1;
            let successor = graph.blocks[block].successors[index];
            if !visited[successor] {
                visited[successor] = true;
                stack.push((successor, 0));
            }
        } else {
            post.push(block);
            stack.pop();
        }
    }
    post.reverse();
    post
}

// cite: art nodes.cc#UpdateDominatorOfSuccessor
pub fn dominators(graph: &Graph, rpo: &[BlockId]) -> Vec<Option<BlockId>> {
    let n = graph.blocks.len();
    let mut rpo_index = vec![usize::MAX; n];
    for (index, block) in rpo.iter().enumerate() {
        rpo_index[*block] = index;
    }
    let mut idom: Vec<Option<BlockId>> = vec![None; n];
    if let Some(entry) = rpo.first() {
        idom[*entry] = Some(*entry);
    }
    let mut changed = true;
    while changed {
        changed = false;
        for &block in rpo {
            if Some(block) == rpo.first().copied() {
                continue;
            }
            let mut new_idom: Option<BlockId> = None;
            for &pred in &graph.blocks[block].predecessors {
                if idom[pred].is_none() {
                    continue;
                }
                new_idom = Some(match new_idom {
                    None => pred,
                    Some(current) => intersect(pred, current, &idom, &rpo_index),
                });
            }
            if idom[block] != new_idom {
                idom[block] = new_idom;
                changed = true;
            }
        }
    }
    idom
}

fn intersect(
    mut a: BlockId,
    mut b: BlockId,
    idom: &[Option<BlockId>],
    rpo_index: &[usize],
) -> BlockId {
    while a != b {
        while rpo_index[a] > rpo_index[b] {
            match idom[a] {
                Some(next) if next != a => a = next,
                _ => break,
            }
        }
        while rpo_index[b] > rpo_index[a] {
            match idom[b] {
                Some(next) if next != b => b = next,
                _ => break,
            }
        }
        if idom[a].is_none() || idom[b].is_none() {
            break;
        }
    }
    a
}

// cite: art nodes.h#Dominates
pub fn dominates(a: BlockId, b: BlockId, idom: &[Option<BlockId>]) -> bool {
    let mut current = Some(b);
    while let Some(node) = current {
        if node == a {
            return true;
        }
        match idom.get(node).copied().flatten() {
            Some(next) if next != node => current = Some(next),
            _ => break,
        }
    }
    false
}

// cite: art nodes.cc#ComputeDominanceInformation, UpdateDominatorOfSuccessor
pub fn dominance_frontiers(graph: &Graph, idom: &[Option<BlockId>]) -> Vec<HashSet<BlockId>> {
    let n = graph.blocks.len();
    let mut df = vec![HashSet::new(); n];
    for block in &graph.blocks {
        if block.predecessors.len() < 2 {
            continue;
        }
        let idom_block = idom[block.id];
        for &pred in &block.predecessors {
            let mut runner = pred;
            while Some(runner) != idom_block {
                match idom[runner] {
                    Some(next) if next != runner => {
                        df[runner].insert(block.id);
                        runner = next;
                    }
                    _ => break,
                }
            }
        }
    }
    df
}

// cite: art ssa_builder.cc#BuildSsa
pub fn phi_sites(graph: &Graph, df: &[HashSet<BlockId>]) -> HashMap<u16, HashSet<BlockId>> {
    let mut def_sites: HashMap<u16, Vec<BlockId>> = HashMap::new();
    for block in &graph.blocks {
        for instr in &block.instructions {
            if let Some(reg) = instr.op.defined_register() {
                def_sites.entry(reg).or_default().push(block.id);
            }
        }
    }
    let mut sites: HashMap<u16, HashSet<BlockId>> = HashMap::new();
    for (reg, defs) in &def_sites {
        let mut phi_blocks: HashSet<BlockId> = HashSet::new();
        let mut worklist = defs.clone();
        while let Some(block) = worklist.pop() {
            for &frontier in &df[block] {
                if phi_blocks.insert(frontier) {
                    worklist.push(frontier);
                }
            }
        }
        sites.insert(*reg, phi_blocks);
    }
    sites
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dex3oat::graph::build;
    use crate::dex3oat::reg_type::RegType;
    use crate::dex3oat::wir::{IfCmp, WirOp};

    fn diamond() -> Graph {
        let wir = vec![
            (
                0,
                WirOp::IfZero {
                    a: 0,
                    cmp: IfCmp::Ne,
                    target: 4,
                },
            ),
            (2, WirOp::ConstInt { dst: 1, value: 10 }),
            (3, WirOp::Goto { target: 5 }),
            (4, WirOp::ConstInt { dst: 1, value: 20 }),
            (
                5,
                WirOp::Return {
                    src: 1,
                    ty: RegType::Int32,
                },
            ),
        ];
        build(&wir, 2).expect("diamond builds")
    }

    #[test]
    fn rpo_starts_at_entry_and_visits_every_block() {
        let g = diamond();
        let rpo = reverse_post_order(&g);
        assert_eq!(rpo.len(), g.blocks.len());
        assert_eq!(rpo[0], g.entry);
    }

    #[test]
    fn merge_block_is_dominated_by_entry_not_a_branch() {
        let g = diamond();
        let rpo = reverse_post_order(&g);
        let idom = dominators(&g, &rpo);
        let merge = g
            .blocks
            .iter()
            .find(|b| b.predecessors.len() == 2)
            .expect("diamond has a merge");
        assert_eq!(idom[merge.id], Some(g.entry));
        let entry_succ = g.blocks[g.entry].successors.clone();
        for s in entry_succ {
            assert_eq!(idom[s], Some(g.entry));
        }
    }

    #[test]
    fn phi_placed_where_a_register_is_defined_on_both_arms() {
        let g = diamond();
        let rpo = reverse_post_order(&g);
        let idom = dominators(&g, &rpo);
        let df = dominance_frontiers(&g, &idom);
        let sites = phi_sites(&g, &df);
        let merge = g
            .blocks
            .iter()
            .find(|b| b.predecessors.len() == 2)
            .expect("merge")
            .id;
        assert!(
            sites.get(&1).map(|s| s.contains(&merge)).unwrap_or(false),
            "v1 (defined on both arms) needs a phi at the merge; sites={sites:?}"
        );
    }

    #[test]
    fn straight_line_needs_no_phis() {
        let wir = vec![
            (0, WirOp::ConstInt { dst: 0, value: 1 }),
            (
                1,
                WirOp::IntBinLit {
                    dst: 0,
                    src: 0,
                    literal: 2,
                    kind: crate::dex3oat::wir::IntBinKind::Add,
                },
            ),
            (
                2,
                WirOp::Return {
                    src: 0,
                    ty: RegType::Int32,
                },
            ),
        ];
        let g = build(&wir, 1).expect("builds");
        let rpo = reverse_post_order(&g);
        let idom = dominators(&g, &rpo);
        let df = dominance_frontiers(&g, &idom);
        let sites = phi_sites(&g, &df);
        assert!(sites.values().all(|s| s.is_empty()), "no merges, no phis");
    }
}
