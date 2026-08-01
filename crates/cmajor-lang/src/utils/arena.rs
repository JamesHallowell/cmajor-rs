use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct Arena<K, V> {
    items: Vec<V>,
    _marker: PhantomData<K>,
}

#[macro_export]
macro_rules! arena_key {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyData(u32);

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
    pub fn push(&mut self, item: V) -> K {
        let key = K::from(KeyData(self.items.len() as u32));
        self.items.push(item);
        key
    }

    pub fn last(&self) -> Option<(K, &V)> {
        self.items
            .last()
            .map(|item| (K::from(KeyData((self.items.len() - 1) as u32)), item))
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
        &self.items[KeyData::from(key).0 as usize]
    }
}

impl<K, V> std::ops::IndexMut<K> for Arena<K, V>
where
    KeyData: From<K>,
{
    fn index_mut(&mut self, key: K) -> &mut Self::Output {
        &mut self.items[KeyData::from(key).0 as usize]
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
            let key = K::from(KeyData(index as u32));
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
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let index = KeyData::from(key).0 as usize;
        if index >= self.items.len() {
            self.items.resize_with(index + 1, || None);
        }
        self.items[index].replace(value)
    }

    pub fn get(&self, key: K) -> Option<&V> {
        self.items.get(KeyData::from(key).0 as usize)?.as_ref()
    }

    pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
        self.items.get_mut(KeyData::from(key).0 as usize)?.as_mut()
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
