use std::{
    fmt::Debug,
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

#[derive(Clone, Copy, Debug)]
pub enum RbNodeType {
    Red,
    Black,
    Nil,
}

/// Base struct for RB-Tree node.
///
/// Nil nodes always have both `None` children.
/// non-Nil nodes have both `RbNode` children in most cases.
/// (in `delete()` method, there's some processes that may temporarily make some child to `None`)
///
/// `key` and `value` are init unless the node is Nil.
///
/// ## Safety
///
/// `key_value_moved` must be correct not to occur double-free or memory leak.
pub struct RawNode<K, V> {
    pub key: MaybeUninit<K>,
    pub value: MaybeUninit<V>,
    pub key_value_moved: bool,
    pub rb_node_type: RbNodeType,
    pub parent: Option<RbNode<K, V>>,
    pub left: Option<RbNode<K, V>>,
    pub right: Option<RbNode<K, V>>,
}

/// Pointer struct for RawNode
///
/// It must be properly dropped using `Box::from_raw`.
pub struct RbNode<K, V>(NonNull<RawNode<K, V>>);

impl<K, V> Deref for RbNode<K, V> {
    type Target = RawNode<K, V>;

    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }
    }
}

impl<K, V> DerefMut for RbNode<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.0.as_mut() }
    }
}

impl<K, V> PartialEq for RbNode<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<K, V> Eq for RbNode<K, V> {}

impl<K, V> Clone for RbNode<K, V> {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl<K, V> Copy for RbNode<K, V> {}

impl<K, V> RbNode<K, V> {
    pub fn new(parent: Option<Self>) -> Self {
        Self(
            NonNull::new(Box::into_raw(Box::new(RawNode {
                key: MaybeUninit::uninit(),
                value: MaybeUninit::uninit(),
                key_value_moved: true,
                rb_node_type: RbNodeType::Nil,
                parent,
                left: None,
                right: None,
            })))
            .unwrap(),
        )
    }

    pub fn as_ptr(&mut self) -> *mut RawNode<K, V> {
        self.0.as_ptr()
    }

    pub fn key(&self) -> &K {
        if self.is_nil() {
            panic!("RbNode should not be Nil");
        }

        unsafe { self.key.assume_init_ref() }
    }

    pub fn value(&self) -> &V {
        if self.is_nil() {
            panic!("RbNode should not be Nil");
        }

        unsafe { self.value.assume_init_ref() }
    }

    pub fn init(&mut self, key: K, value: V, rb_node_type: RbNodeType) {
        if let RbNodeType::Nil = rb_node_type {
            return;
        }

        self.key.write(key);
        self.value.write(value);
        self.key_value_moved = false;

        self.left = Some(RbNode::new(Some(*self)));
        self.right = Some(RbNode::new(Some(*self)));

        self.rb_node_type = rb_node_type;
    }

    pub fn uninit(&mut self) {
        if !self.key_value_moved {
            unsafe {
                self.key.assume_init_drop();
                self.value.assume_init_drop();
            }
            self.key_value_moved = true;
        }

        self.rb_node_type = RbNodeType::Nil;

        if let Some(mut left) = self.left {
            if left.is_nil() {
                unsafe {
                    drop(Box::from_raw(left.as_ptr()));
                }
            } else {
                panic!("Left child is not Nil");
            }
        }

        if let Some(mut right) = self.right {
            if right.is_nil() {
                unsafe {
                    drop(Box::from_raw(right.as_ptr()));
                }
            } else {
                panic!("Right child is not Nil");
            }
        }

        self.left = None;
        self.right = None;
    }

    pub fn is_nil(&self) -> bool {
        match self.rb_node_type {
            RbNodeType::Nil => true,
            _ => false,
        }
    }

    pub fn is_black(&self) -> bool {
        match self.rb_node_type {
            RbNodeType::Red => false,
            _ => true,
        }
    }

    pub fn is_red(&self) -> bool {
        !self.is_black()
    }

    pub fn set_black(&mut self) {
        self.rb_node_type = match self.rb_node_type {
            RbNodeType::Nil => panic!("Modifying Nil is prohibited"),
            _ => RbNodeType::Black,
        };
    }

    pub fn set_red(&mut self) {
        self.rb_node_type = match self.rb_node_type {
            RbNodeType::Nil => panic!("Modifying Nil is prohibited"),
            _ => RbNodeType::Red,
        };
    }
}

impl<K, V> Debug for RbNode<K, V>
where
    K: PartialOrd + Debug,
    V: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let RbNodeType::Nil = self.rb_node_type {
            f.debug_struct(format!("{:?}", &self.rb_node_type).as_str())
                .finish()
        } else {
            f.debug_struct(
                format!(
                    "{:?}({:?},{:?})",
                    &self.rb_node_type,
                    &self.key(),
                    &self.value(),
                )
                .as_str(),
            )
            .finish()
        }
    }
}
