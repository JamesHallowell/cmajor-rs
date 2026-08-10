use {
    crate::{arena_key, ast::NodeId, utils::arena::Arena},
    std::{
        cell::RefCell,
        marker::PhantomData,
        range::RangeInclusive,
        rc::{Rc, Weak},
    },
};

arena_key!(ChildId(pub(super) NodeId));

pub type ChildList = RangeInclusive<ChildId>;

#[derive(Debug, Default, Clone)]
pub struct ChildPool {
    scratch: Rc<RefCell<Vec<NodeId>>>,
    children: Arena<ChildId, NodeId>,
}

pub struct Checkpoint<T>(usize, Weak<RefCell<Vec<NodeId>>>, PhantomData<T>);

pub struct MaybeNoneStaged {}
pub struct AtLeastOneStaged {}

pub trait Commit {
    type Committed;

    fn commit(checkpoint: usize, pool: &mut ChildPool) -> Self::Committed;
}

impl Commit for Checkpoint<MaybeNoneStaged> {
    type Committed = Option<ChildList>;

    fn commit(checkpoint: usize, pool: &mut ChildPool) -> Self::Committed {
        let (mut first, mut last) = (None, None);
        for node in pool.scratch.borrow_mut().drain(checkpoint..) {
            let child = pool.children.push(node);
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
}

impl Commit for Checkpoint<AtLeastOneStaged> {
    type Committed = ChildList;

    fn commit(checkpoint: usize, pool: &mut ChildPool) -> Self::Committed {
        let first = pool
            .children
            .push(pool.scratch.borrow_mut().remove(checkpoint));

        let last = pool
            .scratch
            .borrow_mut()
            .drain(checkpoint..)
            .map(|node| pool.children.push(node))
            .last()
            .unwrap_or(first);

        ChildList::from(first..=last)
    }
}

impl Checkpoint<MaybeNoneStaged> {
    pub fn with_at_least_one_staged(self, node: NodeId) -> Checkpoint<AtLeastOneStaged> {
        self.stage(node);
        Checkpoint(self.0, self.1.clone(), PhantomData)
    }
}

impl<T> Checkpoint<T> {
    pub fn stage(&self, node: NodeId) {
        self.1
            .upgrade()
            .inspect(|scratch| scratch.borrow_mut().push(node));
    }

    pub fn abort(self) {
        if let Some(scratch) = self.1.upgrade() {
            scratch.borrow_mut().truncate(self.0);
        }
    }
}

impl ChildPool {
    pub fn new() -> Self {
        Self {
            scratch: Rc::default(),
            children: Arena::default(),
        }
    }

    pub fn checkpoint(&self) -> Checkpoint<MaybeNoneStaged> {
        Checkpoint(
            self.scratch.borrow().len(),
            Rc::downgrade(&self.scratch),
            PhantomData,
        )
    }

    pub fn commit<T>(
        &mut self,
        Checkpoint(checkpoint, _, _): Checkpoint<T>,
    ) -> <Checkpoint<T> as Commit>::Committed
    where
        Checkpoint<T>: Commit,
    {
        Checkpoint::<T>::commit(checkpoint, self)
    }

    pub fn get(&self, children: ChildList) -> &[NodeId] {
        self.children.slice(children)
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
        checkpoint.stage(node);
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
            checkpoint.stage(node2);
            let children = pool.commit(checkpoint);

            assert_eq!(
                children.map(|children| pool.get(children)),
                Some([node2].as_slice())
            );
        }

        checkpoint.stage(node1);
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
            checkpoint.stage(a1);
            checkpoint.stage(a2);
            let children = pool.commit(checkpoint).unwrap();
            assert_eq!(pool.get(children.clone()), [a1, a2]);
            nodes.push(())
        };

        let group_b = {
            let checkpoint = pool.checkpoint();
            let b1 = nodes.push(());
            let b2 = nodes.push(());
            checkpoint.stage(b1);
            checkpoint.stage(b2);
            let children = pool.commit(checkpoint).unwrap();
            assert_eq!(pool.get(children.clone()), [b1, b2]);
            nodes.push(())
        };

        outer_checkpoint.stage(group_a);
        outer_checkpoint.stage(group_b);
        let children = pool.commit(outer_checkpoint);

        assert_eq!(
            children.map(|children| pool.get(children)),
            Some([group_a, group_b].as_slice())
        );
    }
}
