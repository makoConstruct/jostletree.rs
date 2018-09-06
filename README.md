# JostleTree

A JostleTree is a Partial Sum Tree with item widths stored in the branches of the tree, rather than just the leaves.

The jostletree can be thought of as representing a series of items with variable span, positioned one after the other. If the span of one item changes, the positions of all of the items after it shift along with it. The jostletree supports logarithmic time access at position, resize node, insert node, and remove node operations.

Its main application seems to be in sampling randomly from large sets where each element in the set may have a different probability of being drawn.

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

## Other Possible Applications

It was conceived for the application of storing enormous sequence, or tree UIs, where multiple users could be altering the structure at the same time. Users would be able to view an approximate overview, to jump to arbitrary offsets instantly, all in logorithmic time. Actually doing this might require some concurrency work though

I made this mainly because it pinged my heuristics for potential usefulness and I couldn't find any preexisting implementations. Hopefully others can think of more applications than I can.

## Past work

It's very similar to a Partial Sum Tree as described in [AN EFFICIENT METHOD FOR WEIGHTED SAMPLING WITHOUT REPLACEMENT* C. K. WONG AND M. C. EASTON](https://doi.org/10.1137/0209009). The difference is, it stores elements and element weights in branches instead of in special different leaf nodes, reducing the number of allocations by about half, and reducing the number of accesses per query logarithmically? I'm not sure why anybody would do it the other way.
