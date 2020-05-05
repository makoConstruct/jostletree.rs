# JostleTree

A JostleTree is a Partial Sum Tree with item widths stored in the branches of the tree, rather than just the leaves.

To understand what kinds of things a jostletree is good for, imagine a series of objects of varying sizes on a narrow shelf packed tightly together. There are no spaces between them. When you insert a new item, all of the items after it need to move over a bit. If an item's size changes, again, all of the times after it need to move a bit, it stays tightly packed.

The jostletree supports insertion and resizing and removal, and it also supports random-access-at-position, picking a distance from the start and drawing whatever is there. All operations mentioned here run in logarithmic time.

If you need to model a series of things with those properties, you wont do better than the jostletree. It seems to have one other application, perhaps more practical, of sampling randomly from large sets where each element in the set may have a different probability of being drawn. In that, it can also be used to draw with a bias towards one end or the other.

```rust
let mut candidate = JostleTree::<usize, char>::new();
candidate.insert_back(1, 'a');
candidate.insert_back(9, 'b');
candidate.insert_back(1, 'c');
candidate.insert_back(1, 'd');
assert_eq!(candidate.get_item(5).unwrap(), &'b');
assert_eq!(candidate.get_item(10).unwrap(), &'c');
assert_eq!(candidate.get_item(11).unwrap(), &'d');

candidate.insert_at(5, 1, 'e');
assert_eq!(candidate.get_item(1).unwrap(), &'e');
```

## Using floats as spans

The data structure is generic over span types, but `f32`s and `f64`s wont work because they do not implement `Ord`. (The reason they don't implement Ord is that there exists a float for which neither a < b nor a > b. Can you guess which float it is?. It's `NaN`. `NaN` is also the reason floats can't implement `Eq`. There are some data structures that will actually break and do unsafe things if you give trick them into using floats, for this reason. `NaN`s are pretty horrible, really.)

But fear not. You can just use https://crates.io/crates/noisy_float. It's a no-overhead wrapper around floats that disallows `NaN`s.

## Past work

It's very similar to a Partial Sum Tree as described in [AN EFFICIENT METHOD FOR WEIGHTED SAMPLING WITHOUT REPLACEMENT* C. K. WONG AND M. C. EASTON](https://doi.org/10.1137/0209009). The difference is, it stores elements and element weights in branches instead of in special different leaf nodes, reducing the number of allocations by about half, and reducing the number of accesses per query logarithmically? I'm not sure why anybody would do it the other way.
