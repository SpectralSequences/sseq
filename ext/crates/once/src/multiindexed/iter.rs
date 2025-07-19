use super::{kdtrie::KdTrie, node::Node};

pub struct KdTrieIterator<'a, V> {
    trie: &'a KdTrie<V>,
    stack: Vec<(Vec<i32>, &'a Node<V>, std::ops::Range<i32>)>,
}
impl<'a, V> KdTrieIterator<'a, V> {
    pub(crate) fn new(trie: &'a KdTrie<V>) -> Self {
        let root_range = if trie.dimensions() == 1 {
            unsafe { trie.root().leaf() }.range()
        } else {
            unsafe { trie.root().inner() }.range()
        };
        Self {
            trie,
            stack: vec![(vec![], trie.root(), root_range)],
        }
    }
}

impl<'a, V> Iterator for KdTrieIterator<'a, V> {
    type Item = (Vec<i32>, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((mut current_coords, current_node, mut range)) = self.stack.pop() {
            // Find the next index in the current range that has a value
            while let Some(idx) = range.next() {
                if current_coords.len() == self.trie.dimensions() - 1 {
                    // This is a leaf node, check if there's a value at this index
                    let current_leaf = unsafe { current_node.leaf() };
                    if let Some(value) = current_leaf.get(idx) {
                        // Push back the remaining range for this node
                        if !range.is_empty() {
                            self.stack
                                .push((current_coords.clone(), current_node, range));
                        }

                        // Return the value with full coordinates
                        current_coords.push(idx);
                        return Some((current_coords, value));
                    }
                } else {
                    // This is an inner node, check if there's a child at this index
                    let current_inner = unsafe { current_node.inner() };
                    if let Some(child_node) = current_inner.get(idx) {
                        // Push back the remaining range for this node
                        if !range.is_empty() {
                            self.stack
                                .push((current_coords.clone(), current_node, range));
                        }

                        // Add the current index to coordinates and push the child
                        current_coords.push(idx);
                        let child_range = if current_coords.len() == self.trie.dimensions() - 1 {
                            unsafe { child_node.leaf() }.range()
                        } else {
                            unsafe { child_node.inner() }.range()
                        };
                        self.stack.push((current_coords, child_node, child_range));

                        // Go to the next iteration of the outer loop, which will process the child
                        break;
                    }
                }
            }
        }

        None
    }
}
