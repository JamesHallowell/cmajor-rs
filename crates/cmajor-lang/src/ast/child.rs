use {
    crate::{arena_key, ast::NodeId, utils::arena::Arena},
    std::range::RangeInclusive,
};

arena_key!(ChildId(pub(super) NodeId));

pub type ChildList = RangeInclusive<ChildId>;

#[derive(Debug, Default, Clone)]
pub struct ChildPool {
    scratch: Vec<NodeId>,
    pool: Arena<ChildId, NodeId>,
}

pub struct Checkpoint(usize);

impl ChildPool {
    pub fn new() -> Self {
        Self {
            scratch: Vec::default(),
            pool: Arena::default(),
        }
    }

    pub fn checkpoint(&self) -> Checkpoint {
        Checkpoint(self.scratch.len())
    }

    pub fn stage(&mut self, node: NodeId) {
        self.scratch.push(node);
    }

    pub fn commit(&mut self, Checkpoint(checkpoint): Checkpoint) -> Option<ChildList> {
        if self.scratch.is_empty() {
            return None;
        }

        let (mut first, mut last) = (None, None);
        for node in self.scratch.drain(checkpoint..) {
            let child = self.pool.push(node);
            if first.is_none() {
                first = Some(child);
            }
            last = Some(child);
        }

        match (first, last) {
            (Some(first), Some(last)) => Some(ChildList::from(first..=last)),
            _ => None,
        }
    }

    pub fn get(&self, children: ChildList) -> &[NodeId] {
        self.pool.slice(children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_commit() {
        let mut pool = ChildPool::default();

        let checkpoint = pool.checkpoint();
        let children = pool.commit(checkpoint);

        assert_eq!(children, None);
    }

    #[test]
    fn single_commit() {
        let mut nodes = Arena::<NodeId, ()>::default();
        let mut pool = ChildPool::default();

        let checkpoint = pool.checkpoint();
        let node = nodes.push(());
        pool.stage(node);
        let children = pool.commit(checkpoint);

        assert_eq!(
            children.map(|children| pool.get(children)),
            Some([node].as_slice())
        );
    }

    #[test]
    fn nested_commit() {
        let mut nodes = Arena::<NodeId, ()>::default();
        let mut pool = ChildPool::default();

        let checkpoint = pool.checkpoint();
        let node1 = nodes.push(());

        {
            let checkpoint = pool.checkpoint();
            let node2 = nodes.push(());
            pool.stage(node2);
            let children = pool.commit(checkpoint);

            assert_eq!(
                children.map(|children| pool.get(children)),
                Some([node2].as_slice())
            );
        }

        pool.stage(node1);
        let children = pool.commit(checkpoint);

        assert_eq!(
            children.map(|children| pool.get(children)),
            Some([node1].as_slice())
        );
    }

    #[test]
    fn multiple_sibling_groups_each_with_their_own_nested_commit() {
        let mut nodes = Arena::<NodeId, ()>::default();
        let mut pool = ChildPool::default();

        let outer_checkpoint = pool.checkpoint();

        let group_a = {
            let checkpoint = pool.checkpoint();
            let a1 = nodes.push(());
            let a2 = nodes.push(());
            pool.stage(a1);
            pool.stage(a2);
            let children = pool.commit(checkpoint).unwrap();
            assert_eq!(pool.get(children.clone()), [a1, a2]);
            nodes.push(())
        };

        let group_b = {
            let checkpoint = pool.checkpoint();
            let b1 = nodes.push(());
            let b2 = nodes.push(());
            pool.stage(b1);
            pool.stage(b2);
            let children = pool.commit(checkpoint).unwrap();
            assert_eq!(pool.get(children.clone()), [b1, b2]);
            nodes.push(())
        };

        pool.stage(group_a);
        pool.stage(group_b);
        let children = pool.commit(outer_checkpoint);

        assert_eq!(
            children.map(|children| pool.get(children)),
            Some([group_a, group_b].as_slice())
        );
    }
}
