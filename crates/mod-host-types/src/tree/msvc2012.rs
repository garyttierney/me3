use crate::{
    alloc::DlStdAllocator,
    tree::{Iter, IterMut, RawTree, Tree},
};

#[repr(C)]
pub struct XTree<K, V> {
    raw: RawTree<K, V>,
    alloc: DlStdAllocator,
}

impl<K, V> XTree<K, V> {
    pub fn from_raw_parts(raw: RawTree<K, V>, alloc: DlStdAllocator) -> Self {
        Self { raw, alloc }
    }
}

impl<K, V> Tree<K, V> for XTree<K, V>
where
    K: PartialOrd,
{
    fn get(&self, key: &K) -> Option<&V> {
        self.raw.get(key)
    }

    fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.raw.insert_in(key, value, &self.alloc)
    }

    fn iter(&self) -> Iter<'_, K, V> {
        self.raw.iter()
    }

    fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        self.raw.iter_mut()
    }
}

impl<K, V> Drop for XTree<K, V> {
    fn drop(&mut self) {
        unsafe {
            self.raw.drop_in(&self.alloc);
        }
    }
}
