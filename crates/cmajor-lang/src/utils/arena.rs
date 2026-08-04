use {
    crate::static_assert_eq,
    std::{collections::HashMap, marker::PhantomData, mem::size_of, num::NonZero},
};

#[derive(Debug, Clone)]
pub struct Arena<K, V> {
    items: Vec<V>,
    _marker: PhantomData<K>,
}

#[macro_export]
macro_rules! arena_key {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name($crate::utils::arena::KeyData);

        impl From<$crate::utils::arena::KeyData> for $name {
            fn from(data: $crate::utils::arena::KeyData) -> Self {
                $name(data)
            }
        }

        impl From<$name> for $crate::utils::arena::KeyData {
            fn from(key: $name) -> Self {
                key.0
            }
        }
    };
    ($name:ident($vis:vis $wrapped:ident)) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name($vis $wrapped);

        impl From<$crate::utils::arena::KeyData> for $name {
            fn from(data: $crate::utils::arena::KeyData) -> Self {
                $name($wrapped::from(data))
            }
        }

        impl From<$name> for $crate::utils::arena::KeyData {
            fn from(key: $name) -> Self {
                $crate::utils::arena::KeyData::from(key.0)
            }
        }

        impl From<$name> for $wrapped {
            fn from(key: $name) -> Self {
                key.0
            }
        }
    };
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeyData(NonZero<u32>);

static_assert_eq!(size_of::<Option<KeyData>>(), size_of::<u32>());

impl KeyData {
    fn to_index(&self) -> usize {
        (self.0.get() - 1) as usize
    }

    fn from_index(index: usize) -> Self {
        assert_ne!(index, u32::MAX as usize);
        KeyData(unsafe { NonZero::new_unchecked((index + 1) as u32) })
    }
}

impl std::fmt::Debug for KeyData {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl<K, V> Arena<K, V> {
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.items.iter()
    }
}

impl<K, V> Arena<K, V>
where
    K: From<KeyData>,
{
    pub fn push(&mut self, item: impl Into<V>) -> K {
        let key = K::from(KeyData::from_index(self.items.len()));
        self.items.push(item.into());
        key
    }

    pub fn first(&self) -> (K, &V) {
        let key = K::from(KeyData::from_index(0));
        (key, &self.items[0])
    }

    pub fn last(&self) -> Option<(K, &V)> {
        self.items
            .last()
            .map(|item| (K::from(KeyData::from_index(self.items.len() - 1)), item))
    }
}

impl<K, V> Default for Arena<K, V> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            _marker: PhantomData,
        }
    }
}

impl<K, V> std::ops::Index<K> for Arena<K, V>
where
    KeyData: From<K>,
{
    type Output = V;

    fn index(&self, key: K) -> &Self::Output {
        &self.items[KeyData::from(key).to_index()]
    }
}

impl<K, V> std::ops::IndexMut<K> for Arena<K, V>
where
    KeyData: From<K>,
{
    fn index_mut(&mut self, key: K) -> &mut Self::Output {
        &mut self.items[KeyData::from(key).to_index()]
    }
}

#[derive(Debug, Clone)]
pub struct Iter<'a, K, V> {
    arena: &'a Arena<K, V>,
    remaining: usize,
}

impl<'a, K, V> Iterator for Iter<'a, K, V>
where
    K: From<KeyData>,
{
    type Item = (K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            None
        } else {
            let index = self.arena.items.len() - self.remaining;
            let key = K::from(KeyData::from_index(index));
            self.remaining -= 1;
            Some((key, &self.arena.items[index]))
        }
    }
}

impl<'a, K, V> IntoIterator for &'a Arena<K, V>
where
    K: From<KeyData>,
{
    type Item = (K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            arena: self,
            remaining: self.items.len(),
        }
    }
}

impl<K, V> FromIterator<V> for Arena<K, V> {
    fn from_iter<I: IntoIterator<Item = V>>(iter: I) -> Self {
        Self {
            items: iter.into_iter().collect(),
            _marker: PhantomData,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SecondaryArena<K, V> {
    items: Vec<Option<V>>,
    _marker: PhantomData<K>,
}

impl<K, V> Default for SecondaryArena<K, V> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            _marker: PhantomData,
        }
    }
}

impl<K, V> SecondaryArena<K, V>
where
    KeyData: From<K>,
{
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let index = KeyData::from(key).to_index();
        if index >= self.items.len() {
            self.items.resize_with(index + 1, || None);
        }
        self.items[index].replace(value)
    }

    pub fn get(&self, key: K) -> Option<&V> {
        self.items.get(KeyData::from(key).to_index())?.as_ref()
    }

    pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
        self.items.get_mut(KeyData::from(key).to_index())?.as_mut()
    }
}

impl<K, V> std::ops::Index<K> for SecondaryArena<K, V>
where
    KeyData: From<K>,
{
    type Output = V;

    fn index(&self, key: K) -> &Self::Output {
        self.get(key)
            .expect("no value in SecondaryArena for this key")
    }
}

impl<K, V> std::ops::IndexMut<K> for SecondaryArena<K, V>
where
    KeyData: From<K>,
{
    fn index_mut(&mut self, key: K) -> &mut Self::Output {
        self.get_mut(key)
            .expect("no value in SecondaryArena for this key")
    }
}

impl<K, V> FromIterator<(K, V)> for SecondaryArena<K, V>
where
    KeyData: From<K>,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut arena = Self::default();
        for (k, v) in iter {
            arena.insert(k, v);
        }
        arena
    }
}

#[derive(Debug, Clone)]
pub struct SparseSecondaryArena<K, V> {
    items: HashMap<KeyData, V>,
    _marker: PhantomData<K>,
}

impl<K, V> Default for SparseSecondaryArena<K, V> {
    fn default() -> Self {
        Self {
            items: HashMap::default(),
            _marker: PhantomData,
        }
    }
}

impl<K, V> SparseSecondaryArena<K, V>
where
    KeyData: From<K>,
{
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.items.insert(KeyData::from(key), value)
    }

    pub fn get(&self, key: K) -> Option<&V> {
        self.items.get(&KeyData::from(key))
    }

    pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
        self.items.get_mut(&KeyData::from(key))
    }
}

impl<K, V> std::ops::Index<K> for SparseSecondaryArena<K, V>
where
    KeyData: From<K>,
{
    type Output = V;

    fn index(&self, key: K) -> &Self::Output {
        self.get(key)
            .expect("no value in SparseSecondaryArena for this key")
    }
}

impl<K, V> std::ops::IndexMut<K> for SparseSecondaryArena<K, V>
where
    KeyData: From<K>,
{
    fn index_mut(&mut self, key: K) -> &mut Self::Output {
        self.get_mut(key)
            .expect("no value in SparseSecondaryArena for this key")
    }
}

impl<K, V> FromIterator<(K, V)> for SparseSecondaryArena<K, V>
where
    KeyData: From<K>,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut arena = Self::default();
        for (k, v) in iter {
            arena.insert(k, v);
        }
        arena
    }
}
