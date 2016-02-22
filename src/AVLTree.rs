#![feature(core_intrinsics)]
use std::mem::{uninitialized, replace, forget, transmute};
use std::ptr::null_mut;
use std::cmp::{max, Ord, PartialEq};
use std::iter::Iterator;
use std::fmt::{Display, Formatter, Debug};
use std::hash::{Hash,Hasher};
use std::fmt;

//TODO: refactor, realizing that rust has no tail recursion

//functions that panic on failure in debug mode and have undefined behavior on failure in release mode(but're perfectly efficient(presumably))
#[inline(always)]
fn seriously_unreachable()-> ! {
	if cfg!(debug_assertions) { panic!("this code path is not supposed to be reachable") }
	else{ unsafe{std::intrinsics::unreachable()} }
}
#[inline(always)]
fn seriously_unwrap<T>(v:Option<T>)-> T {
	match v {
		Some(r)=> r,
		None=> seriously_unreachable(),
	}
}

// unsafe fn warp_lifetime<'src, 'tgt, T>(inv:&'src mut T)-> &'tgt mut T {  transmute(inv)  } //genuinely needed for implementing mut iterators

unsafe fn cp_nonoverlapping<T>(src:*const T, dst:*mut T){ std::ptr::copy_nonoverlapping(src,dst,1); }

fn switch_left<T>(a:*mut T, b:*mut T, c:*mut T){
	debug_assert!(a != b && b != c && a != c);
	unsafe{
		let mut swap:T = uninitialized();
		cp_nonoverlapping(a, &mut swap);
		cp_nonoverlapping(b, a);
		cp_nonoverlapping(c, b);
		cp_nonoverlapping(& swap, c);
		forget(swap);
	}
}

unsafe fn rotate_right<A>(root: *mut Nref<A>){ //assumes root and leftofroot are some.
	let rleft:*mut Nref<A> = &mut seriously_unwrap((*root).as_mut()).left;
	let rleftright:*mut Nref<A> = &mut seriously_unwrap((*rleft).as_mut()).right;
	let root_to_be:*mut Node<A> = &mut **seriously_unwrap((*rleft).as_mut());
	let old_root:*mut Node<A> = &mut **seriously_unwrap((*root).as_mut());
	if (*rleftright).is_some() {
		cp_nonoverlapping(& old_root, &mut seriously_unwrap((*rleftright).as_mut()).parent);
	}
	cp_nonoverlapping(& (*old_root).parent, &mut (*root_to_be).parent);
	cp_nonoverlapping(& root_to_be, &mut (*old_root).parent);
	switch_left(root, rleft, rleftright);
	(*old_root).update_deepness();
	(*root_to_be).update_deepness();
}

unsafe fn rotate_left<A>(root: *mut Nref<A>){ //assumes root and rightofroot are some.
	let rright:*mut Nref<A> = &mut seriously_unwrap((*root).as_mut()).right;
	let rrightleft:*mut Nref<A> = &mut seriously_unwrap((*rright).as_mut()).left;
	let root_to_be:*mut Node<A> = &mut **seriously_unwrap((*rright).as_mut());
	let old_root:*mut Node<A> = &mut **seriously_unwrap((*root).as_mut());
	if (*rrightleft).is_some() {
		cp_nonoverlapping(& old_root, &mut seriously_unwrap((*rrightleft).as_mut()).parent);
	}
	cp_nonoverlapping(& (*old_root).parent, &mut (*root_to_be).parent);
	cp_nonoverlapping(&root_to_be, &mut (*old_root).parent);
	switch_left(root, rright, rrightleft);
	(*old_root).update_deepness();
	(*root_to_be).update_deepness();
}


struct Node<T> {
	v: T,
	deepness: u8,
	parent: *mut Node<T>,
	left: Nref<T>,
	right: Nref<T>,
}

type Nref<T> = Option<Box<Node<T>>>;

fn as_ptr<T>(nref:&Nref<T>)-> *const Node<T> {
	unsafe{transmute(nref)}
	// match *nref {
	// 	Some(ref bt)=> (*bt).as_ref(),
	// 	None=> null_ptr(),
	// }
}

fn deepness<A>(nref:&Nref<A>) -> u8{
	match *nref {
		None => 0,
		Some(ref n) => n.deepness
	}
}

fn fresh_terminal_node<T>(v:T, pnode:*mut Node<T>)-> Node<T> { Node{ v:v, deepness:1, parent:pnode, left:None, right:None } }
unsafe fn balance<T>(rootofrotation: *mut Nref<T>){ //assumes nref is some
	let ro:&mut Node<T> = &mut **seriously_unwrap((*rootofrotation).as_mut());
	ro.update_deepness();
	match ro.balance_score() {
		2 => { //the unwraps I do here can be assumed to succeed due to the balance scores
			if seriously_unwrap(ro.right.as_ref()).balance_score() < 0 {
				rotate_right(&mut ro.right);
			}
			rotate_left(rootofrotation);
		}
		-2 => {
			if seriously_unwrap(ro.left.as_ref()).balance_score() > 0 {
				rotate_left(&mut ro.left);
			}
			rotate_right(rootofrotation);
		}
		_ => ()
	}
}
impl<T> Node<T> {
	fn balance_score(&self) -> i32 { deepness(&self.right) as i32 - deepness(&self.left) as i32 }
	fn update_deepness(&mut self){
		self.deepness = max(deepness(&self.left), deepness(&self.right)) + 1;
	}
}

// fn leftmost_child_mut<T>(n: &mut Node<T>)-> &mut Node<T> {
// 	match n.left {
// 		Some(ref mut nl)=> leftmost_child_mut(nl),
// 		None=> n
// 	}
// }
fn leftmost_child<T>(n: &Node<T>)-> &Node<T> {
	match n.left {
		Some(ref nl)=> leftmost_child(nl),
		None=> n
	}
}

pub struct AVLTree<T:Ord>{
	head_node: Nref<T>,
	count:u64,
}
impl<T:PartialEq + Ord> AVLTree<T> {
	pub fn new()-> AVLTree<T> { AVLTree{head_node:None, count:0} }
	pub fn empty(&self)-> bool { self.head_node.is_none() }
	pub fn element_count(&self)-> u64 { self.count }
	pub fn insert(&mut self, v:T){
		fn node_insert<A:Ord>(trees_counter:&mut u64, parent:*mut Node<A>, cn:&mut Nref<A>, v:A){
			let cnp:*mut Nref<A> = cn;
			match *cn {
				Some(ref mut n) => {unsafe{
					if v < n.v {
						node_insert(trees_counter, &mut **n, &mut n.left, v);
						balance(cnp);
					} else if v > n.v {
						node_insert(trees_counter, &mut **n, &mut n.right, v);
						balance(cnp);
					}//else is already present
				}}
				None =>{
					*cn = Some(Box::new(fresh_terminal_node(v, parent)));
					*trees_counter += 1;
				}
			}
		};
		node_insert(&mut self.count, null_mut(), &mut self.head_node, v);
	}
	pub fn remove(&mut self, v:T){
		unsafe fn seeking<T:Ord + PartialEq>(trees_counter:&mut u64, v: T, n: *mut Nref<T>){
			match *n {
				Some(ref mut tn) =>{
					if v == tn.v { //found
						*trees_counter -= 1;
						match (tn.left.is_some(), tn.right.is_some()) {
							(false, false) =>{
								*n = None
							}
							(true, false)  =>{
								*n = tn.left.take();
							}
							(false, true) =>{
								*n = tn.right.take();
							}
							(true, true) =>{
								unsafe fn abduct_eldest_child<T>(rv:&mut T, noderef:&mut Nref<T>){ //finds the eldest child, removes it, replaces tn's v with its, modifying the lineage appropriately. Assumes the nref is some.
									if seriously_unwrap(noderef.as_mut()).right.is_some() {
										abduct_eldest_child(rv, &mut seriously_unwrap(noderef.as_mut()).right);
										balance(noderef);
									} else {
										let childs_room:Box<Node<T>> = seriously_unwrap(replace(noderef, None)); //the child's room is destroyed in the process of the abduction
										*rv = childs_room.v;
									}
								}
								let tn_beyond_the_box = &mut **tn;
								abduct_eldest_child(
									&mut tn_beyond_the_box.v,
									&mut tn_beyond_the_box.left);
							}
						}
					}else if v <= tn.v {
						seeking(trees_counter, v, &mut tn.left);
						balance(n);
					}else{
						seeking(trees_counter, v, &mut tn.right);
						balance(n);
					}
				}
				_ => {}
			}
		}
		unsafe{
			seeking(&mut self.count, v, &mut self.head_node);
		};
	}
	pub fn contains(&self, v:&T)-> bool{
		fn hasing<T:Ord + PartialEq>(cn:&Nref<T>, v:&T)-> bool {
			match *cn {
				Some(ref n) =>{
					let has = n.v == *v;
					if has { true } else { hasing(if *v <= n.v { &n.left }else{ &n.right }, v) }
				}
				None => { false }
			}
		}
		hasing(&self.head_node, v)
	}
	pub fn iter<'a>(&'a self)-> AVLTreeIter<'a, T> {
		match self.head_node {
			Some(ref n) =>{
				AVLTreeIter{current_node:Some(leftmost_child(&**n))}
			}
			None => AVLTreeIter{current_node:None}
		}
	}
}

pub struct AVLTreeIter<'a, T:'a>{current_node: Option<&'a Node<T>>}
impl<'a, T:'a> Iterator for AVLTreeIter<'a, T>{
	type Item = &'a T;
	fn next(&mut self)-> Option<Self::Item> {
		let ret;
		self.current_node = match self.current_node.as_ref() { //I should not be doing this assignment when current_node is None but that's Rust
			Some(curnode)=>{
				let Node{
					ref v,
					ref right, ..
				} = **curnode;
				ret = Some(v);
				match *right {
					Some(ref n) =>{
						Some(leftmost_child(n))
					}
					None =>{
						//ascend right as many times as you have to until you can ascend left, then you're on the correct node
						unsafe{
							let mut upper_maybe = curnode.parent;
							let next_node:Option<&'a Node<T>>;
							loop{
								if upper_maybe != null_mut() {
									if as_ptr(&(*upper_maybe).left) == *curnode as *const _ {
										next_node = Some(transmute(upper_maybe));
										break;
									}else{
										upper_maybe = (*upper_maybe).parent;
									}
								}else{
									next_node = None;
									break;
								}
							}
							next_node
						}
					}
				}
			},
			None=>{
				ret = None;
				None
			}
		};
		ret
	}
}


fn hash_of<H:Hasher, T:Hash>(nr:&Nref<T>, h:&mut H){
	match *nr {
		Some(ref bn)=>{
			bn.v.hash(h);
			hash_of(&bn.left, h);
			hash_of(&bn.right, h);
		},
		None=> {}
	}
}

impl<T:Hash+Ord> Hash for AVLTree<T> {
	fn hash<H:Hasher>(&self, h:&mut H) {
		hash_of(&self.head_node, h);
	}
}


impl<T:PartialEq+Ord> PartialEq for AVLTree<T> {
	fn eq(&self, other:&AVLTree<T>)-> bool {
		self.iter().zip(other.iter()).all(|(l,r)| l == r )
	}
	fn ne(&self, other:&AVLTree<T>)-> bool { ! self.eq(other) }
}

impl<T:Eq+Ord> Eq for AVLTree<T> {}


struct DisplayableNref<'a, T:'a> {  v:&'a Nref<T>  }
impl<'a, T:Debug + Ord + PartialEq> Debug for DisplayableNref<'a, T> {
	fn fmt(&self, f:&mut Formatter)-> fmt::Result {
		match *self.v {
			Some(ref this) => write!(f, "({:?} {:?} {:?})", DisplayableNref{v:&this.left}, &this.v, DisplayableNref{v:&this.right}),
			None => write!(f, "nil"),
		}
	}
}
impl<T:Display + Ord + PartialEq> Display for AVLTree<T> {
	fn fmt(&self, f:&mut Formatter)-> fmt::Result {
		try!(write!(f,"AVLTree["));
		let mut iter = self.iter();
		match iter.next() {
			Some(el)=> {
				try!(write!(f,"{}", el));
				for el in iter {
					try!(write!(f,", {}", el));
				}
			},
			_=> {}
		};
		try!(write!(f,"]"));
		Ok(())
	}
}
	
	
	
	
	
mod test{
	extern crate rand;
	use super::AVLTree;
	use super::Nref;
	
	
	struct PairIterator<I:Iterator> where I::Item:Copy {
		prev:Option<I::Item>, iter:I,
	}
	fn neighbors<I:Iterator>(mut iter:I)-> PairIterator<I> where I::Item:Copy {
		PairIterator{prev:iter.next(), iter:iter}
	}
	impl<I:Iterator> Iterator for PairIterator<I> where I::Item:Copy {
		type Item = (I::Item,I::Item);
		fn next(&mut self)-> Option<(I::Item,I::Item)> {
			match self.prev {
				Some(p)=>{
					let n = self.iter.next();
					let ret = match n {
						Some(nn)=> Some((p,nn)),
						None=> None,
					};
					self.prev = n;
					ret
				},
				None=> { None }
			}
		}
	}
	
	
	fn is_balanced<T:Ord>(tree:&AVLTree<T>)-> bool{
		fn of_node<A>(o:&Nref<A>)-> bool{
			match *o {
				Some(ref n) =>{
					n.balance_score().abs() <= 1 && of_node(&n.left) && of_node(&n.right)
				}
				None => { true }
			}
		}
		of_node(&tree.head_node)
	}
	fn is_sorted<T:Ord>(tree:&AVLTree<T>)-> bool {
		neighbors(tree.iter()).all(|(a,b)| a < b )
	}
	

	#[test]
	fn many_insert_and_chech_has(){
		let mut candidate: AVLTree<u64> = AVLTree::new();
		candidate.insert(2);
		candidate.insert(3);
		candidate.insert(2);
		candidate.insert(4);
		candidate.insert(5);
		candidate.insert(6);
		candidate.insert(10);
		candidate.insert(11);
		candidate.insert(9);
		println!("{}", &candidate);
		assert!(is_balanced(&candidate));
		println!("{}", candidate.element_count());
		assert!(candidate.element_count() == 8);
		assert!(!candidate.contains(&1u64));
		assert!(candidate.contains(&5u64));
		println!("{}", &candidate);
		assert!(candidate.contains(&9u64));
	}
	#[test]
	fn one_redundancy(){
		let mut candidate = AVLTree::<u64>::new();
		candidate.insert(50);
		candidate.insert(40);
		candidate.insert(45);
		candidate.insert(47);
		candidate.insert(47);
		assert!(is_balanced(&candidate));
		println!("{}", &candidate);
		assert!(candidate.element_count() == 4);
	}

	#[test]
	fn redundances(){
		let mut candidate = AVLTree::<u64>::new();
		candidate.insert(2);
		candidate.remove(2);
		candidate.remove(2);
		candidate.insert(3);
		candidate.insert(3);
		assert!(candidate.element_count() == 1);
	}

	#[test]
	fn initially_empty(){
		let candidate = AVLTree::<u64>::new();
		assert!(candidate.empty());
	}

	#[test]
	fn iterating_and_sorting(){
		let mut candidate = AVLTree::<u64>::new();
		candidate.insert(5);
		candidate.insert(9);
		candidate.insert(38);
		candidate.insert(32);
		candidate.insert(0);
		candidate.insert(0);
		let expected_ordering:&[u64] = &[0, 5, 9, 32, 38, 666];
		println!("{}", candidate);
		assert!(candidate.iter().zip(expected_ordering.iter()).all(|(a,b)|{ a == b }));
	}

	#[test]
	fn not_empty_when_given_one(){
		let mut candidate = AVLTree::<u64>::new();
		candidate.insert(1);
		assert!(!candidate.empty());
	}

	#[test]
	fn has_one_when_given_one(){
		let mut candidate = AVLTree::<u64>::new();
		candidate.insert(1);
		assert!(candidate.count == 1);
	}

	#[test]
	fn big_attack(){
		let mut candidate = AVLTree::<u64>::new();
		const SET_SIZE:usize = 700usize;
		const INPUT_SIZE:usize = 1400usize;
		let mut list:[u64; INPUT_SIZE] = unsafe{::std::mem::uninitialized()}; //I just don't consider it all that unsafe :/
		for i in 0usize .. SET_SIZE {
			list[i] = i as u64;
		}
		for i in SET_SIZE .. INPUT_SIZE {
			list[i] = list[rand::random::<usize>()%SET_SIZE];
		}
		for i in 0usize .. INPUT_SIZE - 1 {
			let iplustone = i+1;
			list.swap(i, iplustone + (rand::random::<usize>()%(INPUT_SIZE - iplustone)));
		}
		//list is now a shuffled list with exactly SET_SIZE unique elements and INPUT_SIZE - SET_SIZE repeats
		for i in list.iter() {
			candidate.insert(*i);
		}
		assert!(candidate.count == SET_SIZE as u64);
		let removals = 1000u64;
		for i in 0 .. removals {
			candidate.remove(i);
		}
		assert!(is_balanced(&candidate));
		assert!(is_sorted(&candidate));
	}
}