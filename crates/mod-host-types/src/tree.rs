use std::{
    marker::PhantomData,
    mem::{self, ManuallyDrop, MaybeUninit},
    ptr::NonNull,
};

use me3_mod_protocol::Game;
use rdvec::alloc::Alloc;

use crate::{alloc::DlStdAllocator, game::GAME};

mod msvc2012;
mod msvc2015;

pub trait Tree<K, V>
where
    K: PartialOrd,
{
    fn get(&self, key: &K) -> Option<&V>;

    fn contains(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    fn insert(&mut self, key: K, value: V) -> Option<V>;

    fn iter(&self) -> Iter<'_, K, V>;

    fn iter_mut(&mut self) -> IterMut<'_, K, V>;
}

#[repr(C)]
pub union XTree<K, V> {
    msvc2012: ManuallyDrop<msvc2012::XTree<K, V>>,
    msvc2015: ManuallyDrop<msvc2015::XTree<K, V>>,
}

pub type TreeMap<K, V> = XTree<K, V>;

pub type TreeSet<V> = XTree<V, ()>;

pub struct Iter<'a, K, V> {
    inner: NonNull<TreeNode<K, V>>,
    _marker: PhantomData<&'a XTree<K, V>>,
}

pub struct IterMut<'a, K, V> {
    inner: NonNull<TreeNode<K, V>>,
    _marker: PhantomData<&'a mut XTree<K, V>>,
}

impl<K, V> XTree<K, V> {
    pub fn new() -> Self {
        let alloc = DlStdAllocator::new();
        let head = TreeNode::new_head_in(&alloc);

        let raw = RawTree {
            head,
            size: 0,
            _marker: PhantomData,
        };

        match *GAME {
            game if game < Game::Sekiro => Self {
                msvc2012: ManuallyDrop::new(msvc2012::XTree::from_raw_parts(raw, alloc)),
            },
            _ => Self {
                msvc2015: ManuallyDrop::new(msvc2015::XTree::from_raw_parts(raw, alloc)),
            },
        }
    }
}

impl<K, V> XTree<K, V>
where
    K: PartialOrd,
{
    pub fn as_dyn(&self) -> &dyn Tree<K, V> {
        match *GAME {
            game if game < Game::Sekiro => unsafe { &*self.msvc2012 },
            _ => unsafe { &*self.msvc2015 },
        }
    }

    pub fn as_mut_dyn(&mut self) -> &mut dyn Tree<K, V> {
        match *GAME {
            game if game < Game::Sekiro => unsafe { &mut *self.msvc2012 },
            _ => unsafe { &mut *self.msvc2015 },
        }
    }
}

impl<K, V> Default for XTree<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> Drop for XTree<K, V> {
    fn drop(&mut self) {
        match *GAME {
            game if game < Game::Sekiro => unsafe {
                ManuallyDrop::drop(&mut self.msvc2012);
            },
            _ => unsafe { ManuallyDrop::drop(&mut self.msvc2015) },
        }
    }
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = &'a (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let next = self.inner.as_child_ref()?;
            self.inner.next();
            Some(&next.key_value)
        }
    }
}

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = &'a mut (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let next = self.inner.as_child_mut()?;
            self.inner.next();
            Some(&mut next.key_value)
        }
    }
}

#[repr(C)]
struct RawNode<T, TChild = T> {
    // None of these are ever null, there are no null checks in the STL.
    left: NonNull<RawNode<TChild>>,
    parent: NonNull<RawNode<TChild>>,
    right: NonNull<RawNode<TChild>>,

    // These are `char`s per ABI, but may only ever be 0 or 1.
    is_black: bool,
    is_nil: bool,

    // The head node does not initialize this field, but still includes it.
    key_value: T,
}

// Any node including head (`is_nil` may be true).
type TreeNode<K, V> = RawNode<MaybeUninit<(K, V)>>;

// Child nodes only (`is_nil` is proven false).
type ChildNode<K, V> = RawNode<(K, V), MaybeUninit<(K, V)>>;

#[repr(C)]
struct RawTree<K, V> {
    // The head is always allocated even in an empty tree.
    head: NonNull<TreeNode<K, V>>,

    // Does not count the head node.
    size: usize,

    // Same variance as a key value pair.
    _marker: PhantomData<(K, V)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Child {
    Left,
    Right,
}

struct NodePos<K, V> {
    parent: NonNull<TreeNode<K, V>>,
    bound: NonNull<TreeNode<K, V>>,
    child: Child,
}

impl<K, V> RawTree<K, V>
where
    K: PartialOrd,
{
    fn lower_bound(&self, key: &K) -> NodePos<K, V> {
        // `head->parent` is the root node.
        let mut result = NodePos {
            parent: unsafe { self.head.as_ref().parent },
            bound: self.head,
            child: Child::Right,
        };

        // In an empty tree the first `try_node` will be nil.
        let mut try_node = result.parent;

        while let Some(try_node_ref) = unsafe { try_node.as_child_ref() } {
            result.parent = try_node;

            if try_node_ref.key_value.0 < *key {
                result.child = Child::Right;
                try_node = try_node_ref.right;
            } else {
                result.child = Child::Left;
                result.bound = try_node;
                try_node = try_node_ref.left;
            }
        }

        result
    }
}

impl<K, V> RawTree<K, V> {
    fn get(&self, key: &K) -> Option<&V>
    where
        K: PartialOrd,
    {
        let mut pos = self.lower_bound(key);
        let node = unsafe { pos.bound.as_child_mut()? };
        (*key >= node.key_value.0).then_some(&node.key_value.1)
    }

    fn insert_in(&mut self, key: K, value: V, alloc: &impl Alloc<TreeNode<K, V>>) -> Option<V>
    where
        K: PartialOrd,
    {
        let mut pos = self.lower_bound(&key);

        if let Some(node) = unsafe { pos.bound.as_child_mut() }
            && key >= node.key_value.0
        {
            // Exact key match (already inserted).
            let (_, value) = mem::replace(&mut node.key_value, (key, value));
            Some(value)
        } else {
            unsafe {
                // We will be inserting a new node (attach to head for now).
                let node = TreeNode::new_child_in(key, value, self.head, alloc);
                self.insert_node(node, pos);
                None
            }
        }
    }

    unsafe fn insert_node(&mut self, mut node: NonNull<TreeNode<K, V>>, mut pos: NodePos<K, V>) {
        self.size += 1;

        unsafe {
            node.as_mut().parent = pos.parent;
        }

        if pos.parent == self.head {
            // First node in tree (root).
            let head = unsafe { self.head.as_mut() };

            head.left = node;
            head.parent = node;
            head.right = node;

            // Root is always black.
            unsafe {
                node.as_mut().is_black = true;
            }

            return;
        }

        match pos.child {
            Child::Right => unsafe {
                // Insert right of parent.
                pos.parent.as_mut().right = node;
                if pos.parent == self.head.as_ref().right {
                    // New rightmost (biggest) node.
                    self.head.as_mut().right = node;
                }
            },
            Child::Left => unsafe {
                // Insert left of parent.
                pos.parent.as_mut().left = node;
                if pos.parent == self.head.as_ref().left {
                    // New leftmost (smallest) node.
                    self.head.as_mut().left = node;
                }
            },
        }

        // Need to rebalance and recolor the red-black tree.
        unsafe {
            self.rebalance_at(node);
        }
    }

    /// https://github.com/microsoft/STL/blob/2a62bf7b4079f0a3e33ec80d7daf9ea968805b12/stl/inc/xtree#L707
    unsafe fn rebalance_at(&mut self, mut node: NonNull<TreeNode<K, V>>) {
        unsafe {
            while let mut parent = node.as_ref().parent
                && !parent.as_ref().is_black
            {
                let mut parent_parent = parent.as_ref().parent;
                let mut parent_sibling = parent_parent.as_ref().left;

                if parent == parent_sibling {
                    parent_sibling = parent_parent.as_ref().right;

                    if !parent_sibling.as_ref().is_black {
                        parent.as_mut().is_black = true;
                        parent_sibling.as_mut().is_black = true;
                        parent_parent.as_mut().is_black = false;

                        node = parent_parent;
                    } else {
                        if node == parent.as_ref().right {
                            node = parent;

                            self.rotate_left_at(node);

                            parent = node.as_ref().parent;
                            parent_parent = parent.as_ref().parent;
                        }

                        parent.as_mut().is_black = true;
                        parent_parent.as_mut().is_black = false;

                        self.rotate_right_at(parent_parent);
                    }
                } else {
                    if !parent_sibling.as_ref().is_black {
                        parent.as_mut().is_black = true;
                        parent_sibling.as_mut().is_black = true;
                        parent_parent.as_mut().is_black = false;

                        node = parent_parent;
                    } else {
                        if node == parent.as_ref().left {
                            node = parent;

                            self.rotate_right_at(node);

                            parent = node.as_ref().parent;
                            parent_parent = parent.as_ref().parent;
                        }

                        parent.as_mut().is_black = true;
                        parent_parent.as_mut().is_black = false;

                        self.rotate_left_at(parent_parent);
                    }
                }
            }

            // Root is always black.
            self.head.as_mut().parent.as_mut().is_black = true;
        }
    }

    /// https://github.com/microsoft/STL/blob/2a62bf7b4079f0a3e33ec80d7daf9ea968805b12/stl/inc/xtree#L494
    unsafe fn rotate_left_at(&mut self, mut pos: NonNull<TreeNode<K, V>>) {
        unsafe {
            let mut node = pos.as_ref().right;
            pos.as_mut().right = node.as_ref().left;

            if !node.as_ref().left.as_ref().is_nil {
                node.as_mut().left.as_mut().parent = pos;
            }

            node.as_mut().parent = pos.as_ref().parent;

            if pos == self.head.as_ref().parent {
                self.head.as_mut().parent = node;
            } else if pos == pos.as_ref().parent.as_ref().left {
                pos.as_mut().parent.as_mut().left = node;
            } else {
                pos.as_mut().parent.as_mut().right = node;
            }

            node.as_mut().left = pos;
            pos.as_mut().parent = node;
        }
    }

    /// https://github.com/microsoft/STL/blob/2a62bf7b4079f0a3e33ec80d7daf9ea968805b12/stl/inc/xtree#L516
    unsafe fn rotate_right_at(&mut self, mut pos: NonNull<TreeNode<K, V>>) {
        unsafe {
            let mut node = pos.as_ref().left;
            pos.as_mut().left = node.as_ref().right;

            if !node.as_ref().right.as_ref().is_nil {
                node.as_mut().right.as_mut().parent = pos;
            }

            node.as_mut().parent = pos.as_ref().parent;

            if pos == self.head.as_ref().parent {
                self.head.as_mut().parent = node;
            } else if pos == pos.as_ref().parent.as_ref().right {
                pos.as_mut().parent.as_mut().right = node;
            } else {
                pos.as_mut().parent.as_mut().left = node;
            }

            node.as_mut().right = pos;
            pos.as_mut().parent = node;
        }
    }

    fn iter(&self) -> Iter<'_, K, V> {
        let min = unsafe { self.head.as_ref().left };
        Iter {
            inner: min,
            _marker: PhantomData,
        }
    }

    fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        let min = unsafe { self.head.as_mut().left };
        IterMut {
            inner: min,
            _marker: PhantomData,
        }
    }

    unsafe fn drop_in(&mut self, alloc: &impl Alloc<TreeNode<K, V>>) {
        unsafe fn drop_recursive_in<K, V>(
            mut node: NonNull<TreeNode<K, V>>,
            alloc: &impl Alloc<TreeNode<K, V>>,
        ) {
            let (left, right) = unsafe {
                let node = node.as_mut();

                if node.is_nil {
                    // We're back at the head node.
                    return;
                }

                // Drop the child node key value pair.
                node.key_value.assume_init_drop();

                (node.left, node.right)
            };

            unsafe {
                let _ = alloc.dealloc(NonNull::slice_from_raw_parts(node, 1));
            }

            unsafe {
                // Drop left and right subtrees.
                drop_recursive_in(left, alloc);
                drop_recursive_in(right, alloc);
            }
        }

        unsafe {
            let root = self.head.as_mut().parent;
            drop_recursive_in(root, alloc);

            // The head node is trivial to drop.
            let _ = alloc.dealloc(NonNull::slice_from_raw_parts(self.head, 1));
        }
    }
}

impl<K, V> TreeNode<K, V> {
    fn new_head_in(alloc: &impl Alloc<Self>) -> NonNull<Self> {
        Self::new_in(None, alloc)
    }

    fn new_child_in(
        key: K,
        value: V,
        head: NonNull<Self>,
        alloc: &impl Alloc<Self>,
    ) -> NonNull<Self> {
        let mut node = Self::new_in(Some((key, value)), alloc);

        unsafe {
            let node = node.as_mut();
            node.left = head;
            node.parent = head;
            node.right = head;
        }

        node
    }

    fn new_in(key_value: Option<(K, V)>, alloc: &impl Alloc<Self>) -> NonNull<Self> {
        unsafe {
            let node = alloc.alloc(1).unwrap().cast::<Self>();
            node.write(Self {
                left: node,
                parent: node,
                right: node,
                is_black: false,
                is_nil: key_value.is_none(),
                key_value: key_value
                    .map(MaybeUninit::new)
                    .unwrap_or_else(MaybeUninit::uninit),
            });
            node
        }
    }
}

trait TreeNodePtrExt<K, V> {
    unsafe fn as_child_ref<'a>(&self) -> Option<&'a ChildNode<K, V>>;

    unsafe fn as_child_mut<'a>(&mut self) -> Option<&'a mut ChildNode<K, V>>;

    unsafe fn next(&mut self);
}

impl<K, V> TreeNodePtrExt<K, V> for NonNull<TreeNode<K, V>> {
    unsafe fn as_child_ref<'a>(&self) -> Option<&'a ChildNode<K, V>> {
        unsafe { (!self.as_ref().is_nil).then(|| self.cast().as_ref()) }
    }

    unsafe fn as_child_mut<'a>(&mut self) -> Option<&'a mut ChildNode<K, V>> {
        unsafe { (!self.as_ref().is_nil).then(|| self.cast().as_mut()) }
    }

    /// https://github.com/microsoft/STL/blob/2a62bf7b4079f0a3e33ec80d7daf9ea968805b12/stl/inc/xtree#L61
    unsafe fn next(&mut self) {
        unsafe {
            if self.as_ref().right.as_ref().is_nil {
                let mut parent;

                while {
                    parent = self.as_ref().parent;
                    !parent.as_ref().is_nil
                } && *self == parent.as_ref().right
                {
                    *self = parent;
                }

                *self = parent;
            } else {
                *self = self.as_ref().right;

                while let left = self.as_ref().left
                    && !left.as_ref().is_nil
                {
                    *self = left;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{array, env, sync::Once};

    use crate::tree::TreeMap;

    fn new_map<K, V>() -> TreeMap<K, V> {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| unsafe { env::set_var("ME3_GAME_LAUNCHED", r#""EldenRing""#) });
        TreeMap::new()
    }

    #[test]
    fn get_empty() {
        let map = new_map::<(), ()>();
        assert_eq!(map.as_dyn().get(&()), None);
    }

    #[test]
    fn insert_simple() {
        let mut map = new_map::<i32, i32>();
        let map = map.as_mut_dyn();

        for i in (0..10).rev() {
            map.insert(i, i);
        }

        for i in 0..10 {
            assert!(map.contains(&i));
        }
    }

    #[test]
    fn sorted_iter() {
        let mut map = new_map::<i32, i32>();
        let map = map.as_mut_dyn();

        for i in (0..10).rev() {
            map.insert(i, i);
        }

        let sorted = map.iter().cloned().collect::<Vec<_>>();
        let expected: [_; 10] = array::from_fn(|i| (i as i32, i as i32));

        assert_eq!(sorted, expected);
    }
}
