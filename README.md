# JostleTree

The jostletree supports insertion and resizing and removal, and it also supports random-access-at-position; picking a distance from the start and drawing whatever is there. All operations mentioned here run in logarithmic time.

A good way to understand what a jostletree is good for is to imagine it as a series of objects of varying sizes on a narrow shelf that are always packed tightly together. There are no spaces between them. You can insert a new item anywhere, and all of the items after it will slide over a bit to accomodate it. You can change the size of one of the items, again, all of the items after will move a bit, it stays tightly packed.

One of its notable applications is sampling randomly from large sequences of elements where each element in the set may have a different probability of being drawn. It is the only solution I'm aware of for random weighted sampling with removal. You can also bias samplings to prefer taking from one end of the list or the other, for instance, if you wanted to sample a content aggregator, so that higher rated, and more recent posts are more likely to be picked, you could sort the contents of the jostletree by age, then draw at `posts.get_item(oldest_allowed*random().powf(3.0))`.

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

## How does it work?

Basically, imagine a binary tree where each branch stores a `width` value, that is the sum of the widths of its children, terminating at the leaves, where the widths are whatever you set them to be. You can navigate quickly to the child at a particular offset by looking at your two children and seeing whether the left one is larger or smaller than you need and navigating down depending on that. There are additional details and optimizations, but this gets you a basic understanding.

(Those optimizations are: Items are stored in the branches, there is no distinct leaf type. Each Branch has two width values, the span of their item, and the total_span of their children. We could further optimize it by (instead?) storing the total_span of the left child, so that the left child does not have to be dereferenced if the search should proceed into the right child, but I wont; if we're serious about optimizing it, branches should be bigger, they should hold however many items will fit in a cache line.)

## Using floats as spans

The data structure is generic over span types, but `f32`s and `f64`s wont work because they do not implement `Ord`. (The reason they don't implement Ord is that there exists a float for which neither a < b nor a >= b. Can you guess which float it is?. It's `NaN`. `NaN` is also the reason floats can't implement `Eq`. There are some data structures that will actually break and do unsafe things if you give trick them into using floats, for this reason. `NaN`s are pretty horrible, really.)

But fear not. You can just use https://crates.io/crates/noisy_float. It's a no-overhead wrapper around floats that disallows `NaN`s.

## Related data structures

It's very similar to a Partial Sum Tree as described in [AN EFFICIENT METHOD FOR WEIGHTED SAMPLING WITHOUT REPLACEMENT* C. K. WONG AND M. C. EASTON](https://doi.org/10.1137/0209009). The difference is, it stores elements and element weights in branches instead of in special different leaf nodes, reducing the number of allocations by about half, and reducing the number of accesses per query logarithmically? I'm not sure why anybody would do it the other way.

The [Alias Method](https://en.wikipedia.org/wiki/Alias_method) seems to be a faster alternative for random weighted sampling, but as far as I can tell, it doesn't look like it supports fast edits (it has to rebuild?). So it wont do random sampling with removal, there may be other things it wont do.
