use std::collections::{BTreeMap, BTreeSet};

pub(super) enum ReferenceError<K> {
    SelfReference { node: K },
    UnknownReference { node: K, reference: K },
}

pub(super) fn validate_references<'a, K>(
    graph: impl IntoIterator<Item = (&'a K, &'a BTreeSet<K>)>,
) -> Result<(), ReferenceError<K>>
where
    K: Clone + Ord + 'a,
{
    let graph = graph.into_iter().collect::<BTreeMap<_, _>>();
    for (node, references) in &graph {
        for reference in (*references).iter() {
            if reference == *node {
                return Err(ReferenceError::SelfReference {
                    node: (*node).clone(),
                });
            }
            if !graph.contains_key(reference) {
                return Err(ReferenceError::UnknownReference {
                    node: (*node).clone(),
                    reference: reference.clone(),
                });
            }
        }
    }
    Ok(())
}

pub(super) fn find_exact_cycle<'a, K>(
    graph: impl IntoIterator<Item = (&'a K, &'a BTreeSet<K>)>,
) -> Option<BTreeSet<K>>
where
    K: Clone + Ord + 'a,
{
    fn visit<K: Clone + Ord>(
        node: &K,
        graph: &BTreeMap<&K, &BTreeSet<K>>,
        complete: &mut BTreeSet<K>,
        stack: &mut Vec<K>,
        active: &mut BTreeMap<K, usize>,
    ) -> Option<BTreeSet<K>> {
        if complete.contains(node) {
            return None;
        }
        if let Some(start) = active.get(node) {
            return Some(stack[*start..].iter().cloned().collect());
        }
        active.insert(node.clone(), stack.len());
        stack.push(node.clone());
        if let Some(references) = graph.get(node) {
            for reference in *references {
                if let Some(cycle) = visit(reference, graph, complete, stack, active) {
                    return Some(cycle);
                }
            }
        }
        stack.pop();
        active.remove(node);
        complete.insert(node.clone());
        None
    }

    let graph = graph.into_iter().collect::<BTreeMap<_, _>>();
    let mut complete = BTreeSet::new();
    for node in graph.keys() {
        if let Some(cycle) = visit(
            *node,
            &graph,
            &mut complete,
            &mut Vec::new(),
            &mut BTreeMap::new(),
        ) {
            return Some(cycle);
        }
    }
    None
}

pub(super) fn cyclic_members<'a, K>(
    graph: impl IntoIterator<Item = (&'a K, &'a BTreeSet<K>)>,
    candidates: &BTreeSet<K>,
) -> BTreeSet<K>
where
    K: Clone + Ord + 'a,
{
    fn reaches<K: Clone + Ord>(
        current: &K,
        target: &K,
        graph: &BTreeMap<&K, &BTreeSet<K>>,
        allowed: &BTreeSet<K>,
        visited: &mut BTreeSet<K>,
    ) -> bool {
        for reference in graph
            .get(current)
            .expect("cycle search only visits known graph nodes")
            .iter()
        {
            if reference == target {
                return true;
            }
            if allowed.contains(reference)
                && visited.insert(reference.clone())
                && reaches(reference, target, graph, allowed, visited)
            {
                return true;
            }
        }
        false
    }

    let graph = graph.into_iter().collect::<BTreeMap<_, _>>();
    candidates
        .iter()
        .filter(|node| reaches(*node, *node, &graph, candidates, &mut BTreeSet::new()))
        .cloned()
        .collect()
}
