mod node;

use self::node::{RbNode, RbNodeType};

use std::{
    borrow::Borrow,
    fmt::Debug,
    marker::PhantomData,
    ops::{Index, IndexMut},
};

/// A sorted map implemented with RB-Tree.
///
/// It maintains RB-Tree attributes when inserting and removing nodes from tree.
///
/// # Examples
///
/// ```
/// use rbtree::RbTree;
///
/// let mut movie_reviews = RbTree::new();
///
/// // review some movies.
/// movie_reviews.insert("Office Space", "Deals with real issues in the workplace.");
/// movie_reviews.insert("Pulp Fiction", "Masterpiece.");
/// movie_reviews.insert("The Godfather", "Very enjoyable.");
/// movie_reviews.insert("The Blues Brothers", "Eye lyked it a lot.");
///
/// // check for a specific one.
/// if !movie_reviews.contains_key("Les Misérables") {
///     println!(
///         "We've got {} reviews, but Les Misérables ain't one.",
///         movie_reviews.len()
///     );
/// }
///
/// // oops, this review has a lot of spelling mistakes, let's delete it.
/// movie_reviews.remove("The Blues Brothers");
///
/// // look up the values associated with some keys.
/// let to_find = ["Up!", "Office Space"];
/// for movie in &to_find {
///     match movie_reviews.get(movie) {
///         Some(review) => println!("{movie}: {review}"),
///         None => println!("{movie} is unreviewed."),
///     }
/// }
///
/// // Look up the value for a key (will panic if the key is not found).
/// println!("Movie review: {}", movie_reviews["Office Space"]);
///
/// // iterate over everything.
/// for (movie, review) in &movie_reviews {
///     println!("{movie}: \"{review}\"");
/// }
/// ```
pub struct RbTree<K, V> {
    root: RbNode<K, V>,
    len: usize,
}

impl<K, V> RbTree<K, V> {
    /// Create new empty RB-Tree
    pub fn new() -> Self {
        Self {
            root: RbNode::new(None),
            len: 0,
        }
    }

    /// Insert a node with key, value.
    ///
    /// if there was duplicate key, replaces with new value and returns previous value.
    /// Otherwise returns `None`.
    pub fn insert(&mut self, key: K, value: V) -> Option<V>
    where
        K: Ord,
    {
        let mut cur = self.find_nearest_node(&key);

        if !cur.is_nil() {
            let old_value = unsafe { cur.value.assume_init_read() };
            cur.value.write(value);

            return Some(old_value);
        }

        cur.init(key, value, RbNodeType::Red);
        self.len += 1;

        // loop case 1 to 3: reassign colors
        loop {
            let (mut parent, mut grand_parent, mut uncle) = match cur.parent {
                Some(parent) => {
                    // case 2: parent is Black
                    if parent.is_black() {
                        return None;
                    }

                    let grand_parent = parent.parent.unwrap();

                    let uncle = if grand_parent.left == Some(parent) {
                        grand_parent.right.unwrap()
                    } else {
                        grand_parent.left.unwrap()
                    };

                    (parent, grand_parent, uncle)
                }

                // case 1: parent is None (cur is root)
                None => {
                    cur.set_black();
                    return None;
                }
            };

            // case 3-2: uncle is Black -> break loop
            if uncle.is_black() {
                break;
            }

            // case 3: uncle is Red (already parent is Red)
            parent.set_black();
            uncle.set_black();
            grand_parent.set_red();

            cur = grand_parent;
        }

        let parent = cur.parent.unwrap();
        let grand_parent = parent.parent.unwrap();

        // case 4: align Red nodes
        if (Some(cur) == parent.right) && (Some(parent) == grand_parent.left) {
            self.rotate_left(parent);
            cur = parent;
        } else if (Some(cur) == parent.left) && (Some(parent) == grand_parent.right) {
            self.rotate_right(parent);
            cur = parent;
        }

        let mut parent = cur.parent.unwrap();
        let mut grand_parent = parent.parent.unwrap();

        //case 5: final rotation
        parent.set_black();
        grand_parent.set_red();

        if Some(cur) == parent.left {
            self.rotate_right(grand_parent);
        } else {
            self.rotate_left(grand_parent);
        }

        None
    }

    /// Removes a node by key and returns its value.
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        match self.remove_entry(key) {
            None => None,
            Some((_, value)) => Some(value),
        }
    }

    /// Removes a node by key and returns its key-value pair.
    pub fn remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let mut target = self.find_nearest_node(key);

        if target.is_nil() {
            return None;
        }

        let right_min = RbTree::min_node(target.right.unwrap());

        let removed_key_value = unsafe {
            (
                target.key.assume_init_read(),
                target.value.assume_init_read(),
            )
        };

        if !right_min.is_nil() {
            unsafe {
                target.key.write(right_min.key.assume_init_read());
                target.value.write(right_min.value.assume_init_read());

                target = right_min;
            }
        }

        target.key_value_moved = true;

        let mut child = match target.left.unwrap().is_nil() {
            true => target.right.unwrap(),
            false => target.left.unwrap(),
        };

        // replace target to child
        child.parent = target.parent;

        if let Some(mut parent) = target.parent {
            if parent.left == Some(target) {
                parent.left = Some(child);
            } else {
                parent.right = Some(child);
            }
        } else {
            self.root = child;
        }

        if target.left.unwrap() == child {
            target.left = None;
        } else {
            target.right = None;
        }

        let target_rb_node_type = target.rb_node_type;

        // release target
        target.uninit();
        let _ = unsafe { Box::from_raw(target.as_ptr()) };

        self.len -= 1;

        match target_rb_node_type {
            RbNodeType::Red => return Some(removed_key_value),
            RbNodeType::Black => {
                if child.is_red() {
                    child.set_black();
                    return Some(removed_key_value);
                }
            }
            _ => unreachable!(),
        }

        let mut node = child;
        let mut parent;
        let mut sibling;

        loop {
            parent = match node.parent {
                Some(parent) => parent,
                // case 1: node is root
                None => return Some(removed_key_value),
            };

            sibling = if parent.left == Some(node) {
                parent.right.unwrap()
            } else {
                parent.left.unwrap()
            };

            // case 2: if sibling is Red, swap colors and rotate parent
            if sibling.is_red() {
                parent.set_red();
                sibling.set_black();

                if parent.left == Some(node) {
                    self.rotate_left(parent);
                } else {
                    self.rotate_right(parent);
                }
            }

            sibling = if parent.left == Some(node) {
                parent.right.unwrap()
            } else {
                parent.left.unwrap()
            };

            // case 3: if all parent, sibling, sibling_left, sibling_right are Black
            // change sibling's color to Red and loop, otherwise break
            if parent.is_black()
                && sibling.is_black()
                && sibling.left.unwrap().is_black()
                && sibling.right.unwrap().is_black()
            {
                sibling.set_red();
                node = parent;
            } else {
                break;
            }
        }

        // case 4: if same to case 3 but parent is Red, swap color of parent and sibling
        if parent.is_red()
            && sibling.is_black()
            && sibling.left.unwrap().is_black()
            && sibling.right.unwrap().is_black()
        {
            sibling.set_red();
            parent.set_black();

            return Some(removed_key_value);
        }

        // case 5
        if sibling.is_black() {
            if parent.left == Some(node)
                && sibling.right.unwrap().is_black()
                && sibling.left.unwrap().is_red()
            {
                sibling.set_red();
                sibling.left.unwrap().set_black();
                self.rotate_right(sibling);
            } else if parent.right == Some(node)
                && sibling.left.unwrap().is_black()
                && sibling.right.unwrap().is_red()
            {
                sibling.set_red();
                sibling.right.unwrap().set_black();
                self.rotate_left(sibling);
            }
        }

        sibling = if parent.left == Some(node) {
            parent.right.unwrap()
        } else {
            parent.left.unwrap()
        };

        // case 6 increase black count in `node`
        sibling.rb_node_type = parent.rb_node_type;
        parent.set_black();

        if parent.left == Some(node) {
            sibling.right.unwrap().set_black();
            self.rotate_left(parent);
        } else {
            sibling.left.unwrap().set_black();
            self.rotate_right(parent);
        }

        Some(removed_key_value)
    }

    /// find the node with key or Nil node with proper place to insert.
    fn find_nearest_node<Q>(&self, key: &Q) -> RbNode<K, V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let mut cur = self.root;

        loop {
            if cur.is_nil() {
                break;
            }

            match cur.key() {
                x if key < x.borrow() => {
                    cur = cur.left.unwrap();
                }
                x if key > x.borrow() => {
                    cur = cur.right.unwrap();
                }
                _ => break,
            }
        }

        cur
    }

    /// find left-most non-Nil node starting from input node.
    ///
    /// It returns Nil only if input node is Nil.
    fn min_node(node: RbNode<K, V>) -> RbNode<K, V> {
        if node.is_nil() {
            return node;
        }

        let mut cur = node;

        loop {
            let left = cur.left.unwrap();
            if left.is_nil() {
                break;
            }

            cur = left;
        }

        cur
    }

    /// find right-most non-Nil node starting from input node.
    ///
    /// It returns Nil only if input node is Nil.
    fn max_node(node: RbNode<K, V>) -> RbNode<K, V> {
        if node.is_nil() {
            return node;
        }

        let mut cur = node;

        loop {
            let right = cur.right.unwrap();
            if right.is_nil() {
                break;
            }

            cur = right;
        }

        cur
    }

    /// Rotate tree to left from input node.
    ///
    /// # Panics
    ///
    /// Panics if `node.right` is `None`
    fn rotate_left(&mut self, mut node: RbNode<K, V>) {
        let mut right = node.right.expect("Right Child should not be None");

        let parent = node.parent;

        if let Some(mut right_left) = right.left {
            right_left.parent = Some(node);
        }

        node.right = right.left;
        node.parent = Some(right);
        right.left = Some(node);
        right.parent = parent;

        if let Some(mut parent) = parent {
            if parent.left == Some(node) {
                parent.left = Some(right);
            } else {
                parent.right = Some(right);
            }
        } else {
            self.root = right;
        }
    }

    /// Rotate tree to right from input node.
    ///
    /// # Panics
    ///
    /// Panics if `node.left` is `None` that is node is Nil
    fn rotate_right(&mut self, mut node: RbNode<K, V>) {
        let mut left = node.left.expect("Left Child should not be None");

        let parent = node.parent;

        if let Some(mut left_right) = left.right {
            left_right.parent = Some(node);
        }

        node.left = left.right;
        node.parent = Some(left);
        left.right = Some(node);
        left.parent = parent;

        if let Some(mut parent) = parent {
            if parent.right == Some(node) {
                parent.right = Some(left);
            } else {
                parent.left = Some(left);
            }
        } else {
            self.root = left;
        }
    }

    /// Check if the tree complies RB-Tree attributes.
    pub fn is_correct_rb_tree(&self) -> bool {
        match RbTree::check_rb_tree_attribute(self.root) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    fn check_rb_tree_attribute(node: RbNode<K, V>) -> Result<u64, ()> {
        let left_black_count = match node.left {
            Some(left) => RbTree::check_rb_tree_attribute(left)?,
            None => 0,
        };

        let right_black_count = match node.right {
            Some(right) => RbTree::check_rb_tree_attribute(right)?,
            None => 0,
        };

        if left_black_count != right_black_count {
            return Err(());
        }

        let self_black_count = match node.is_black() {
            true => 1,
            false => 0,
        };

        Ok(left_black_count + self_black_count)
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns a reference to the value corresponding to the key.
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let mut target = self.find_nearest_node(key);
        match target.is_nil() {
            true => None,
            false => Some(unsafe { (*target.as_ptr()).value.assume_init_ref() }),
        }
    }

    /// Returns a reference to the key-value pair corresponding to the key.
    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let mut target = self.find_nearest_node(key);

        match target.is_nil() {
            true => None,
            false => Some(unsafe {
                (
                    (*target.as_ptr()).key.assume_init_ref(),
                    (*target.as_ptr()).value.assume_init_ref(),
                )
            }),
        }
    }

    /// Returns a mutable reference to the value corresponding to the key.
    pub fn get_mut<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let mut target = self.find_nearest_node(key);

        match target.is_nil() {
            true => None,
            false => Some(unsafe { (*target.as_ptr()).value.assume_init_mut() }),
        }
    }

    /// Check if tree has a node with input key.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        !self.find_nearest_node(key).is_nil()
    }

    /// Makes the tree empty.
    ///
    /// root node turns into Nil node.
    pub fn clear(&mut self) {
        if self.root.is_nil() {
            return;
        }

        let mut stack = vec![self.root];

        while !stack.is_empty() {
            let mut cur = stack.pop().unwrap();

            let (left_is_nil, right_is_nil) =
                (cur.left.unwrap().is_nil(), cur.right.unwrap().is_nil());

            if left_is_nil && right_is_nil {
                cur.uninit();
            } else {
                stack.push(cur);

                if !right_is_nil {
                    stack.push(cur.right.unwrap());
                }

                if !left_is_nil {
                    stack.push(cur.left.unwrap());
                }
            }
        }
    }

    /// Removes left-most node and returns key-value pair
    pub fn pop_first(&mut self) -> Option<(K, V)>
    where
        K: Ord,
    {
        let target = RbTree::min_node(self.root);

        match target.is_nil() {
            true => None,
            false => self.remove_entry(target.key()),
        }
    }

    /// Removes right-most node and returns key-value pair
    pub fn pop_last(&mut self) -> Option<(K, V)>
    where
        K: Ord,
    {
        let target = RbTree::max_node(self.root);

        match target.is_nil() {
            true => None,
            false => self.remove_entry(target.key()),
        }
    }
}

impl<K, V> Debug for RbTree<K, V>
where
    K: Ord + Debug,
    V: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RbTree")
            .field("root", &self.root)
            .field("len", &self.len)
            .finish()
    }
}

impl<K, Q, V> Index<&Q> for RbTree<K, V>
where
    K: Borrow<Q>,
    Q: Ord + ?Sized,
{
    type Output = V;

    fn index(&self, index: &Q) -> &Self::Output {
        let mut target = self.find_nearest_node(index);

        match target.is_nil() {
            true => panic!("key not found"),
            false => unsafe { (*target.as_ptr()).value.assume_init_ref() },
        }
    }
}

impl<K, Q, V> IndexMut<&Q> for RbTree<K, V>
where
    K: Borrow<Q>,
    Q: Ord + ?Sized,
{
    fn index_mut(&mut self, index: &Q) -> &mut Self::Output {
        let mut target = self.find_nearest_node(index);

        match target.is_nil() {
            true => panic!("key not found"),
            false => unsafe { (*target.as_ptr()).value.assume_init_mut() },
        }
    }
}

impl<K, V> Drop for RbTree<K, V> {
    fn drop(&mut self) {
        self.clear();
        unsafe {
            drop(Box::from_raw(self.root.as_ptr()));
        }
    }
}

fn iter_next<K, V>(
    cur: RbNode<K, V>,
    stack: &mut Vec<RbNode<K, V>>,
) -> Option<(RbNode<K, V>, RbNode<K, V>)> {
    let mut cur = cur;

    while !stack.is_empty() || !cur.is_nil() {
        if !cur.is_nil() {
            stack.push(cur);
            cur = cur.left.unwrap();
        } else {
            cur = stack.pop().unwrap();

            return Some((cur.right.unwrap(), cur));
        }
    }

    None
}

pub struct Iter<'a, K, V> {
    cur: RbNode<K, V>,
    stack: Vec<RbNode<K, V>>,
    _marker: PhantomData<(&'a K, &'a V)>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        match iter_next(self.cur, &mut self.stack) {
            Some((cur, mut next)) => {
                self.cur = cur;
                unsafe {
                    Some((
                        (*next.as_ptr()).key.assume_init_ref(),
                        (*next.as_ptr()).value.assume_init_ref(),
                    ))
                }
            }
            None => None,
        }
    }
}

impl<'a, K, V> IntoIterator for &'a RbTree<K, V> {
    type Item = (&'a K, &'a V);

    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        Iter {
            cur: self.root,
            stack: Vec::new(),
            _marker: PhantomData,
        }
    }
}

pub struct IterMut<'a, K, V> {
    cur: RbNode<K, V>,
    stack: Vec<RbNode<K, V>>,
    _marker: PhantomData<(&'a K, &'a mut V)>,
}

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        match iter_next(self.cur, &mut self.stack) {
            Some((cur, mut next)) => {
                self.cur = cur;
                unsafe {
                    Some((
                        (*next.as_ptr()).key.assume_init_ref(),
                        (*next.as_ptr()).value.assume_init_mut(),
                    ))
                }
            }
            None => None,
        }
    }
}

impl<'a, K, V> IntoIterator for &'a mut RbTree<K, V> {
    type Item = (&'a K, &'a mut V);

    type IntoIter = IterMut<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        IterMut {
            cur: self.root,
            stack: Vec::new(),
            _marker: PhantomData,
        }
    }
}

pub struct IntoIter<K, V> {
    _rb_tree: RbTree<K, V>,
    cur: RbNode<K, V>,
    stack: Vec<RbNode<K, V>>,
}

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        match iter_next(self.cur, &mut self.stack) {
            Some((cur, mut next)) => {
                self.cur = cur;
                next.key_value_moved = true;
                unsafe {
                    Some((
                        (*next.as_ptr()).key.assume_init_read(),
                        (*next.as_ptr()).value.assume_init_read(),
                    ))
                }
            }
            None => None,
        }
    }
}

impl<K, V> IntoIterator for RbTree<K, V> {
    type Item = (K, V);

    type IntoIter = IntoIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        let cur = self.root;

        IntoIter {
            _rb_tree: self, // To prevent rb_tree from drop
            cur,
            stack: Vec::new(),
        }
    }
}
