An AVL Tree for rust. Acts like a set. Includes an item count.
The code is somewhat unidiomatic. Rust's borrow tracker didn't quite permit the kind of pointer-bending one wants to do when writing performance-critical code, so I have used raw pointers in a lot of places.

Unit test coverage is provided. [`rustc --test AVLTree.rs && ./AVLTree`]