# JostleTree

A JostleTree is a new (I think?) data structure for working with long lines of tightly packed items with variable widths.

In other words, the JostleTree can be thought of as efficiently modelling a sequence of items of variable widths. It allows operations such as jumping to a position and getting whatever item is there, and, resizing items, in so doing, repositioning every one of the items after it. It does this in logarithmic time.

The positions of the elements are effectively distributed throughout the tree. Each node of the tree stores the sum width of all of the elements underneath it.

Don't hesitate to ask if you want an API feature added, I'll get to it ASAP. There are a few fairly trivial things I haven't done yet because I don't need them myself yet, and it'll be less work if it's done after non-lexical lifetimes is stabilized.


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


## Possible Applications

* It was conceived for the application of storing enormous sequence, or tree UIs, where multiple users could be altering the structure at the same time. Users would be able to view an approximate overview, to jump to arbitrary offsets instantly, all in logorithmic time. This may require a parallel implementation though =/

* Drawing randomly from a large set of weighted elements, such that the probability of drawing a particular element is proportional to its weight, and the weights of each element can change, and where new elements can be added and removed. I know of no other method for doing this efficiently. If you don't want each element to be weighted individually, the JostleTree also provides indexing by order, as if it were an array with efficient insertion. It would make it very easy to draw with a bias towards elements at the front.

* I don't know. Largely I just made this because it pinged my heuristics for potential usefulness and I couldn't find any preexisting implementations. Hopefully others can think of more applications than I can.

## Past work

It's very similar to a Partial Sum Tree as described in [AN EFFICIENT METHOD FOR WEIGHTED SAMPLING WITHOUT REPLACEMENT* C. K. WONG AND M. C. EASTON](https://doi.org/10.1137/0209009). The difference is, it stores elements and element weights in branches instead of in special different leaf nodes, reducing the number of allocations by about half, and reducing the number of accesses per query logarithmically? I'm not sure why anybody would do it the other way.