# JostleTree

A JostleTree is a new (I think?) data structure for working with long lines of tightly packed items with variable widths.

In other words, the JostleTree can be thought of as efficiently modelling a sequence of items of variable widths. It allows operations such as jumping to a position and getting whatever item is there, and, resizing items, in so doing, repositioning every one of the items after it. It does this in logarithmic time.

The positions of the elements are effectively distributed throughout the tree. Each node of the tree stores the sum width of all of the elements underneath it.

Don't hesitate to ask if you want an API feature added, I'll get to it ASAP. There are a few fairly trivial things I haven't done yet because I don't need them myself yet, and it'll be less work if it's done after non-lexical lifetimes is stabilized.

## Possible Applications

* It was conceived for the application of storing enormous sequence, or tree UIs, where multiple users could be altering the structure at the same time. Users would be able to view an approximate overview, to jump to arbitrary offsets instantly, all in logorithmic time. This may require a parallel implementation though =/

* Drawing randomly from a large set of weighted elements, such that the probability of drawing a particular element is proportional to its weight, and the weights of each element can change, and where new elements can be added and removed. I know of no other method for doing this efficiently. If you don't want each element to be weighted individually, the JostleTree also provides indexing by order, as if it were an array with efficient insertion. It would make it very easy to draw with a bias towards elements at the front, basically, `jostle_tree.`

* I don't know. Largely I just made this because it pinged my heuristics for potential usefulness and I couldn't find any preexisting implementations. Hopefully others can think of more applications than I can.

Unit test coverage is provided. [`cargo test`]