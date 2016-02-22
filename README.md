#JostleTree

A JostleTree is a new(I think?) data structure for working with long lines of tightly packed spans of variable widths.
In other words, the JostleTree consists of a sequence of items of variable widths. It allows operations such as going to an item at a particular offset, resizing it, and in so doing, repositioning every one of the items after it. It does this in logarithmic time.
The reason it is able to do this, is that the offsets of individual elements are not stored, they are effectively distributed through the tree.

###Possible Applications

* Taking random draws of a large set of weighted elements, such that the probability of drawing a particular element is proportional to its weight, and the weights of each element can change, and where new elements can be added and removed. If you don't want each element to be weighted individually, the JostleTree also provides indexing by order(as if it were an array with ), and a count of the number of items.
* It was conceived for the application of storing enormous sequence, or tree UIs, where multiple users could be altering the structure at the same time. Users would be able to view an approximate overview, to jump to arbitrary offsets instantly, all in logorithmic time. This may require a parallel implementation though =/
* I don't know. Largely I just made this because it pinged my heuristics for potential usefulness and I couldn't find any preexisting implementations. Hopefully others can think of more applications than I can.

##AVLTree:
I ended up fixing most of the bugs this had in the jostlertree, and I cannot be bothered back-porting the fixes just now. Maybe later. It acted like a set and it included an item count.

Unit test coverage is provided. [`cargo test`]