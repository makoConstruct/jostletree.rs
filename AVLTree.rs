#![feature(macro_rules)]
use std::mem::{uninitialized, forget, transmute};
use std::ptr::{copy_nonoverlapping_memory};
use std::cmp::{max, Ord, PartialEq};
use std::fmt::{Show, Formatter};
use std::fmt;
use std::rand::random;

//this code is not idiomatic, I am using raw pointers all over the place because the borrow checker is far too overbearing for low level code.

type Nref<T> = Option<Box<Node<T>>>;

fn deepness<A>(nref:&Nref<A>) -> u8{
	match *nref {
		None => 0,
		Some(ref n) => n.deepness
	}
}

//basically just unwrap but with an debug_assert instead of matching and failing on None. In release mode, this will be more efficient and less safe than unwrap.
#[inline(always)]
unsafe fn wrest<T>(o:Option<Box<T>>)-> Box<T> {debug_assert!(o.is_some()); transmute(o)}
#[inline(always)]
unsafe fn wrest_ref<'a, T>(o:Option<&'a T>)-> &'a T {debug_assert!(o.is_some()); transmute(o)}
#[inline(always)]
unsafe fn wrest_mut<'a, T>(o:Option<&'a mut T>)-> &'a mut T {debug_assert!(o.is_some()); transmute(o)}

fn switch_left<T>(a:*mut T, b:*mut T, c:*mut T){
	debug_assert!(a != b && b != c && a != c);
	unsafe{
		let mut spare:T = uninitialized();
		copy_nonoverlapping_memory(&mut spare, &*a, 1);
		copy_nonoverlapping_memory(a, &*b, 1);
		copy_nonoverlapping_memory(b, &*c, 1);
		copy_nonoverlapping_memory(c, &spare, 1);
		forget(spare);
	}
}

unsafe fn rotate_right<A>(root: *mut Nref<A>){ //assumes root and leftofroot are some.
	let rleft:*mut Nref<A> = &mut wrest_mut((*root).as_mut()).left;
	let rleftright:*mut Nref<A> = &mut wrest_mut((*rleft).as_mut()).right;
	let root_to_be:*mut Node<A> = &mut **wrest_mut((*rleft).as_mut());
	let old_root:*mut Node<A> = &mut **wrest_mut((*root).as_mut());
	switch_left(root, rleft, rleftright);
	(*old_root).update_deepness();
	(*root_to_be).update_deepness();
}

unsafe fn rotate_left<A>(root: *mut Nref<A>){ //assumes root and leftofroot are some.
	let rright:*mut Nref<A> = &mut wrest_mut((*root).as_mut()).right;
	let rrightleft:*mut Nref<A> = &mut wrest_mut((*rright).as_mut()).left;
	let root_to_be:*mut Node<A> = &mut **wrest_mut((*rright).as_mut());
	let old_root:*mut Node<A> = &mut **wrest_mut((*root).as_mut());
	switch_left(root, rright, rrightleft);
	(*old_root).update_deepness();
	(*root_to_be).update_deepness();
}



// unsafe fn rotate_left<A>(root: &mut Option<Box<Node<A>>>){ //assumes root and leftofroot are some.
// 	let rright = &mut wrest_mut(root.as_mut()).right;
// 	switch_left(root, rright, &mut wrest_mut(rright.as_mut()).left);
// 	let rootnode = wrest_mut(root.as_mut());
// 	wrest_mut(rootnode.right.as_mut()).update_deepness();
// 	rootnode.update_deepness();
// }

struct Node<T> {
	v: T,
	deepness: u8,
	left: Nref<T>,
	right: Nref<T>,
}
fn fresh_terminal_node<T>(v:T)-> Node<T> { Node{ v:v, deepness:1, left:None, right:None } }
unsafe fn balance<T>(rootofrotation: *mut Nref<T>){ //assumes nref is some
	let ro:&mut Node<T> = &mut **wrest_mut((*rootofrotation).as_mut());
	ro.update_deepness();
	match ro.balance_score() {
		2 => { //the wrests I do here can be assumed to succeed due to the balance scores
			if wrest_ref(ro.right.as_ref()).balance_score() < 0 {
				rotate_right(&mut ro.right);
			}
			rotate_left(rootofrotation);
		}
		-2 => {
			if wrest_ref(ro.left.as_ref()).balance_score() > 0 {
				rotate_left(&mut ro.left);
			}
			rotate_right(rootofrotation);
		}
		_ => ()
	}
}
impl<T> Node<T> {
	fn balance_score(&self) -> int { deepness(&self.right) as int - deepness(&self.left) as int }
	fn update_deepness(&mut self){
		self.deepness = max(deepness(&self.left), deepness(&self.right)) + 1;
	}
}
struct AVLTree<T:std::cmp::Ord>{
	head_node: Nref<T>,
	count:uint,
}
impl<T:PartialEq + Ord> AVLTree<T> {
	pub fn new()-> AVLTree<T> { AVLTree{head_node:None, count:0} }
	pub fn empty(&self)-> bool { self.head_node.is_none() }
	pub fn element_count(&self)-> uint { self.count }
	pub fn is_balanced(&self)-> bool{
		fn of_node<A>(o:&Nref<A>)-> bool{
			match *o {
				Some(ref n) =>{
					n.balance_score().abs() <= 1 && of_node(&n.left) && of_node(&n.right)
				}
				None => { true }
			}
		}
		of_node(&self.head_node)
	}
	pub fn insert(&mut self, v:T){
		unsafe fn node_insert<A:Ord>(trees_counter:&mut uint, cn:*mut Nref<A>, v:A){
			match *cn {
				Some(ref mut n) =>{
					if v < n.v {
						node_insert(trees_counter, &mut n.left, v);
						balance(cn);
					}else if v > n.v {
						node_insert(trees_counter, &mut n.right, v);
						balance(cn);
					}//else is already present
				}
				None =>{
					*cn = Some(box fresh_terminal_node(v));
					*trees_counter += 1;
				}
			}
		};
		unsafe{
			node_insert(&mut self.count, &mut self.head_node, v);
		}
	}
	pub fn remove(&mut self, v:T){
		unsafe fn seeking<T:Ord + PartialEq>(trees_counter:&mut uint, v: T, n: *mut Nref<T>){
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
									match wrest_mut(noderef.as_mut()).right {
										Some(_) =>{
											abduct_eldest_child(rv, &mut wrest_mut(noderef.as_mut()).right);
											balance(noderef);
										}
										None =>{
											*rv = wrest(noderef.take()).v;
										}
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
					n.v == *v || hasing(if *v <= n.v { &n.left }else{ &n.right }, v)
				}
				None => { false }
			}
		}
		hasing(&self.head_node, v)
	}
}

struct ShowableNref<'a, T:'a> {
	v:&'a Nref<T>
}
impl<'a, T:Show> Show for ShowableNref<'a, T> { //can't implement Show for an Option. Option already has a Show.
	fn fmt(&self, f:&mut Formatter)-> fmt::Result {
		match *self.v {
			Some(ref this) => write!(f, "({} {} {})", ShowableNref{v:&this.left}, &this.v, ShowableNref{v:&this.right}),
			None => write!(f, "nil"),
		}
	}
}

impl<T:Show + Ord + PartialEq> Show for AVLTree<T> {
	fn fmt(&self, f:&mut Formatter)-> fmt::Result {
		write!(f,"{}", ShowableNref{v:&self.head_node})
	}
}

#[test]
fn many_insert_and_chech_has(){
	let mut candidate: AVLTree<uint> = AVLTree::new();
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
	assert!(candidate.is_balanced());
	println!("{}", candidate.element_count());
	assert!(candidate.element_count() == 8);
	assert!(!candidate.contains(&1u));
	assert!(candidate.contains(&5u))
	println!("{}", &candidate);
	assert!(candidate.contains(&9u));
}

#[test]
fn one_redundancy(){
	let mut candidate = AVLTree::<uint>::new();
	candidate.insert(50);
	candidate.insert(40);
	candidate.insert(45);
	candidate.insert(47);
	candidate.insert(47);
	assert!(candidate.is_balanced());
	println!("{}", &candidate);
	assert!(candidate.element_count() == 4);
}

#[test]
fn redundances(){
	let mut candidate = AVLTree::<uint>::new();
	candidate.insert(2);
	candidate.remove(2);
	candidate.remove(2);
	candidate.insert(3);
	candidate.insert(3);
	assert!(candidate.element_count() == 1);
}

#[test]
fn initially_empty(){
	let candidate = AVLTree::<uint>::new();
	assert!(candidate.empty());
}

#[test]
fn not_empty_when_given_one(){
	let mut candidate = AVLTree::<uint>::new();
	candidate.insert(1);
	assert!(!candidate.empty());
}

#[test]
fn has_one_when_given_one(){
	let mut candidate = AVLTree::<uint>::new();
	candidate.insert(1);
	assert!(candidate.count == 1);
}

#[test]
fn big_attack(){
	let mut candidate = AVLTree::<uint>::new();
	static SET_SIZE:uint = 700u;
	static INPUT_SIZE:uint = 1400u;
	let mut list:[uint, ..INPUT_SIZE] = unsafe{uninitialized()}; //I just don't consider it all that unsafe :/
	for i in range(0u, SET_SIZE) {
		list[i] = i;
	}
	for i in range(SET_SIZE, INPUT_SIZE) {
		list[i] = list[random::<uint>()%SET_SIZE];
	}
	for i in range(0u, INPUT_SIZE - 1) {
		let iplustone = i+1;
		list.swap(i, iplustone + (random::<uint>()%(INPUT_SIZE - iplustone)));
	}
	//list is now a shuffled list with exactly SET_SIZE unique elements and INPUT_SIZE - SET_SIZE repeats
	for &mut i in list.iter() {
		candidate.insert(i);
	}
	assert!(candidate.is_balanced());
	assert!(candidate.count == SET_SIZE);
	let removals = 1000u;
	for i in range(0, removals) {
		candidate.remove(i);
	}
	assert!(candidate.is_balanced());
}