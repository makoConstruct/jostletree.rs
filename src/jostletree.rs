use std::hash::{Hash, Hasher};
use std::mem::{uninitialized, forget, transmute, transmute_copy, replace};
use std::ptr::{copy_nonoverlapping, null_mut, read};
use std::fmt;
use std::ops::{Add, Sub};
use std::fmt::{Display, Formatter};
use std::cmp::{max, Eq, Ord};
use std::iter::Iterator;



#[inline(always)]
unsafe fn warp_ptr_into_mut<'a, T>(v:*mut T)-> &'a mut T { transmute(v) }

#[inline(always)]
unsafe fn nonoverlapping_write<T>(v:T, dst:&mut T){
	copy_nonoverlapping(&v, dst, 1);
	forget(v);
}


// ///warning: if f panics t will be freed twice. f must not panic.
// #[inline(always)]
// fn replace_self<T, F:FnOnce(T)->T> (t:&mut T, f:F){
// 	unsafe{
// 		let fr = f(read(t));
// 		nonoverlapping_write(fr, t);
// 	}
// }

#[inline(always)]
fn replace_self_and_return<T, B, F:FnOnce(T)-> (T,B)> (t:&mut T, f:F)-> B {
	unsafe{
		let (fr, ret) = f(read(t));
		nonoverlapping_write(fr, t);
		ret
	}
}

///serious functions panic on failure in debug mode and have undefined behavior on failure in release mode (but in this case they're perfectly efficient)
#[inline(always)]
fn seriously_unreach(s:&str)-> ! {
	if cfg!(debug_assertions) { panic!("{}", s) }
	else{ unsafe{::std::intrinsics::unreachable()} }
}
#[inline(always)]
fn seriously_unwrap<T>(v:Option<T>)-> T {
	match v {
		Some(r)=> r,
		None=> seriously_unreach("this option must not be None"),
	}
}

// unsafe fn warp_lifetime<'src, 'tgt, T>(inv:&'src T)-> &'tgt T {  transmute(inv)  }
unsafe fn warp_lifetime_mut<'src, 'tgt, T>(inv:&'src mut T)-> &'tgt mut T {  transmute(inv)  }

unsafe fn cp_nonoverlapping<T>(src:*const T, dst:*mut T){ copy_nonoverlapping(src,dst,1); }

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


unsafe fn warp_nref<'a,N,T>(v:*mut *mut Branch<N,T>)-> &'a mut Nref<N,T> { transmute(v) }
unsafe fn warp_bptr<N,T>(v:*mut Nref<N,T>)-> *mut *mut Branch<N,T> { transmute(v) }
unsafe fn warp_bptr_const<N,T>(v:*const Nref<N,T>)-> *const *const Branch<N,T> { transmute(v) }
unsafe fn warp_nullable<N,T>(v:*mut Nref<N,T>)-> *mut Branch<N,T> { transmute_copy(&mut *v) }

pub trait Numeric : Ord + Eq + Add<Output=Self> + Sub<Output=Self> + Clone + Default {}

impl<X> Numeric for X where X : Ord + Eq + Add<Output=Self> + Sub<Output=Self> + Clone + Default {}

unsafe fn rotate_right<N,A>(old_root: *mut *mut Branch<N,A>) where N:Numeric { //assumes root and leftofroot are some.
	let orp = *old_root;
	let rleft:*mut Nref<N,A> = &mut (*orp).left;
	let rleftright:*mut Nref<N,A> = &mut seriously_unwrap((*rleft).as_mut()).right;
	let ol_widdly = warp_nullable(rleftright);
	let root_to_be:*mut Branch<N,A> = &mut **seriously_unwrap((*rleft).as_mut());
	let old_root_as_nref:*mut Nref<N,A> = warp_nref(old_root);
	if !ol_widdly.is_null() {
		cp_nonoverlapping(old_root as *const _, &mut (*ol_widdly).parent);
	}
	cp_nonoverlapping(& (*orp).parent, &mut (*root_to_be).parent);
	cp_nonoverlapping(& root_to_be, &mut (*orp).parent);
	switch_left(old_root_as_nref, rleft, rleftright);
	let old_span = (*orp).total_span.clone();
	let rising_span = (*root_to_be).total_span.clone();
	(*root_to_be).total_span = old_span.clone();
	(*orp).total_span = old_span.clone() -
		if ol_widdly.is_null() {
			rising_span.clone()
		}else{
			rising_span - (*ol_widdly).total_span.clone()
		};
	(*orp).update_deepness();
	(*orp).update_count();
	(*root_to_be).update_deepness();
	(*root_to_be).update_count();
}

unsafe fn rotate_left<N,A>(old_root: *mut *mut Branch<N,A>) where N:Numeric { //assumes root and rightofroot are some.
	let orp = *old_root;
	let rright:*mut Nref<N,A> = &mut (*orp).right;
	let rrightleft:*mut Nref<N,A> = &mut seriously_unwrap((*rright).as_mut()).left;
	let ol_widdly = warp_nullable(rrightleft);
	let root_to_be:*mut Branch<N,A> = &mut **seriously_unwrap((*rright).as_mut());
	let old_root_as_nref:*mut Nref<N,A> = warp_nref(old_root);
	if !ol_widdly.is_null() {
		cp_nonoverlapping(old_root as *const _, &mut (*ol_widdly).parent);
	}
	cp_nonoverlapping(& (*orp).parent, &mut (*root_to_be).parent);
	cp_nonoverlapping(&root_to_be, &mut (*orp).parent);
	switch_left(old_root_as_nref, rright, rrightleft);
	let old_span = (*orp).total_span.clone();
	let rising_span = (*root_to_be).total_span.clone();
	(*root_to_be).total_span = old_span.clone();
	(*orp).total_span = old_span -
		if ol_widdly.is_null() {
			rising_span
		}else{
			rising_span - (*ol_widdly).total_span.clone()
		};
	(*orp).update_deepness();
	(*orp).update_count();
	(*root_to_be).update_deepness();
	(*root_to_be).update_count();
}

#[derive(Debug)]
pub struct Branch<N,T> {
	parent: *mut Branch<N,T>,
	left: Nref<N,T>,
	right: Nref<N,T>,
	pub v:T,
	deepness:u8,
	count:usize,
	span:N,
	total_span:N,
}
type Nref<N,T> = Option<Box<Branch<N,T>>>;
fn eq_branch<N,T>(v:&Nref<N,T>, other:*const Branch<N,T>)-> bool { (other == unsafe{*warp_bptr_const(v)}) }

fn deepness<N:Numeric,T>(nref:&Nref<N,T>)-> u8 {
	match *nref { Some(ref bbs) => bbs.deepness, None => 0 } }
fn count<N:Numeric,T>(nref:&Nref<N,T>)-> usize {
	match *nref { Some(ref bbs)=> bbs.count, None=> 0 } }
fn total_span<N:Numeric,T>(nref:&Nref<N,T>)-> N {
	match *nref { Some(ref bbs)=> bbs.total_span.clone(), None=> N::default() } }

unsafe fn balance<N:Numeric,T>(rootofrotation: *mut *mut Branch<N,T>){
	//OPTIMIZING: Might not always be necessary to update all three scores each time balance is called
	let ro:&mut Branch<N,T> = &mut **rootofrotation;
	ro.update_deepness();
	ro.update_count();
	ro.update_total_span();
	match ro.balance_score() {
		2 => { //the unwraps I do here can be assumed to succeed due to the balance scores
			let rrb = warp_bptr(&mut ro.right);
			if (&**rrb).balance_score() < 0 {
				rotate_right(rrb);
			}
			rotate_left(rootofrotation);
		}
		-2 => {
			let rlb = warp_bptr(&mut ro.left);
			if (&**rlb).balance_score() > 0 {
				rotate_left(rlb);
			}
			rotate_right(rootofrotation);
		}
		_ => ()
	}
}

fn leftmost_child<N,T>(n: &Branch<N,T>)-> &Branch<N,T> {
	match n.left {
		Some(ref nl)=> leftmost_child(nl),
		None=> n
	}
}
fn leftmost_child_mut<N,T>(n: &mut Branch<N,T>)-> &mut Branch<N,T> {
	match n.left {
		Some(ref mut nl)=> leftmost_child_mut(nl),
		None=> n
	}
}

/// The JostleTree can be thought of as efficiently modelling a sequence of items of variable widths. It allows operations such as
///
/// * jumping to a position and getting whatever item is there
///
/// * resizing items, in so doing, repositioning every one of the items after it
///
/// * inserting and removing
///
///
/// Operations generally have logarithmic runtime.
#[derive(Debug)]
pub struct JostleTree<N, T>{
	head_node: Nref<N, T>,
}
impl<N, T> JostleTree<N, T> where N:Numeric {
	pub fn new()-> JostleTree<N, T> { JostleTree{head_node:None} }
	pub fn is_empty(&self)-> bool { self.head_node.is_none() }
	pub fn len(&self)-> usize { count(&self.head_node) as usize }
	/// returns the sum of the spans of all of the items (logarithmic runtime)
	pub fn total_span(&self)-> N { total_span(&self.head_node) }
	unsafe fn create_at_and_balance_from<'a>(head_node:*mut Nref<N,T>, v:T, span:N, rcn:&'a mut Nref<N,T>, parent:*mut Branch<N,T>)-> SlotHandle<'a,N,T> {
		let mut bb = box Branch{v:v, span:span.clone(), deepness:1, count:1, total_span:span, parent:parent, left:None, right:None};
		let r = warp_lifetime_mut(&mut *bb); //we know that the location of Branch doesn't change, it doesn't get freed, and that it lives as long as the return value.
		*rcn = Some(bb);
		let mut p = parent;
		while !p.is_null() {
			balance(parents_mut(head_node, &mut*p));
			p = (*p).parent;
		}
		SlotHandle{head:head_node, v:r}
	}
	/// inserts at or before whatever is at_offset.
	pub fn insert_at<'a>(&'a mut self, at_offset:N, span:N, v:T)-> SlotHandle<'a,N,T>{
		let mut parent = null_mut();
		let head:*mut _ = &mut self.head_node;
		let mut cn:&mut Nref<N,T> = &mut self.head_node;
		let mut offset = at_offset;
		loop{
			let fcn:*mut _ = match *cn {
				Some(ref mut n)=> {
					let bspan = total_span(&n.left) + n.span.clone();
					n.total_span = n.total_span.clone() + span.clone();
					if offset < bspan {
						parent = &mut **n;
						&mut n.left
					} else {
						parent = &mut **n;
						offset = offset - bspan;
						&mut n.right
					}
				}
				ref mut rcn@None=> {
					return unsafe{ JostleTree::create_at_and_balance_from(head, v, span.clone(), rcn, parent) }
				}
			};
			cn = unsafe{warp_ptr_into_mut(fcn)};
		}
	}
	
	#[inline(always)]
	fn insert_where<'a>(&'a mut self, span:N, v:T, front:bool)-> SlotHandle<'a,N,T>{
		let mut parent = null_mut();
		let head:*mut _ = &mut self.head_node;
		let mut cn:&mut Nref<N,T> = &mut self.head_node;
		loop{
			let fcn:*mut _ = match *cn {
				Some(ref mut n)=> {
					n.total_span = n.total_span.clone() + span.clone();
					parent = &mut **n;
					if front { &mut n.left } else { &mut n.right }
				}
				ref mut rcn@None=> {
					return unsafe{ JostleTree::create_at_and_balance_from(head, v, span.clone(), rcn, parent) }
				}
			};
			cn = unsafe{warp_ptr_into_mut(fcn)};
		}
	}
	
	/// inserts at the back
	pub fn insert_back<'a>(&'a mut self, span:N, v:T)-> SlotHandle<'a,N,T>{
		self.insert_where(span,v,false)
	}
	
	/// inserts at the front
	pub fn insert_front<'a>(&'a mut self, span:N, v:T)-> SlotHandle<'a,N,T>{
		self.insert_where(span,v,true)
	}
	
	
	//good version: Does not work because the borrowck is an IMBICILE, so I'll keep this around for nll
	// fn branch_at_offset_mut(&mut self, mut o:N)-> &mut Branch<T> { //negative or out of bounds o values will get first and last thing respectively
	// 	let mut c = &mut **seriously_unwrap(self.head_node.as_mut()); //sufficed by the previous two lines
	// 	loop{
	// 		let lts = total_span(&c.left);
	// 		if o < lts {
	// 			match c.left {
	// 				Some(ref mut lb)=> c = &mut **lb,
	// 				None=> return c,
	// 			}
	// 		}else{
	// 			let rts = lts + c.span;
	// 			if o < rts { return c }
	// 			else {
	// 				match c.right {
	// 					Some(ref mut rb)=> c = &mut **rb,
	// 					None=> return c,
	// 				}
	// 				o -= rts;
	// 			}
	// 		}
	// 	}
	// }
	
	/// returns the branch at the offset o
	///
	/// negative or out of bounds o values will get first and last thing respectively. returns None if tree is empty.
	fn branch_at_offset_mut(&mut self, mut o:N)-> Option<&mut Branch<N,T>> {
		//see above for the reasonable version
		unsafe{//alright, you know what, rust? I'm going to use a fucking pointer for this until you figure out how to infer basic limits to borrows, because I've tried everything that makes sense and none of it is enough for you.
			let mut c:*mut Branch<N,T> = match self.head_node {
				Some(ref mut br)=> &mut **br,
				None=> return None,
			};
			loop{
				let lts = total_span(&(*c).left);
				if o < lts {
					c = match (*c).left {
						Some(ref mut cl)=> &mut **cl,
						None=> return Some(&mut *c),
					};
				}else{
					let rts = lts + (*c).span.clone();
					if o < rts { return Some(&mut *c) }
					else {
						c = match (*c).right {
							Some(ref mut rb)=> &mut **rb,
							None=> return Some(&mut *c),
						};
						o = o - rts;
					}
				}
			}
		}
	}
	
	fn branch_at_offset(&self, mut o:N)-> Option<&Branch<N,T>> { //copy of the above
		//see above for the reasonable version
		unsafe{//alright, you know what, rust? I'm going to use a fucking pointer for this until you figure out how to infer basic limits to borrows, because I've tried everything that makes sense and none of it is enough for you.
			let mut c:*const Branch<N,T> = match self.head_node {
				Some(ref br)=> & **br,
				None=> return None,
			};
			loop{
				let lts = total_span(&(*c).left);
				if o < lts {
					c = match (*c).left {
						Some(ref cl)=> & **cl,
						None=> return Some(& *c),
					};
				}else{
					let rts = lts + (*c).span.clone();
					if o < rts { return Some(& *c) }
					else {
						c = match (*c).right {
							Some(ref rb)=> & **rb,
							None=> return Some(& *c),
						};
						o = o - rts;
					}
				}
			}
		}
	}
	
	/// returns the bucket at the offset o
	///
	/// negative or out of bounds o values will get first and last thing respectively. returns None if tree is empty.
	pub fn get_slot_mut<'a>(&'a mut self, o:N)-> Option<SlotHandle<'a,N,T>> { //None if tree is empty
		let head_ptr:*mut _ = &mut self.head_node;
		self.branch_at_offset_mut(o).map(move|bof| SlotHandle{head:head_ptr, v:bof})
	}
	
	pub fn get_item(&self, o:N)-> Option<&T> { //None if tree is empty
		self.branch_at_offset(o).map(|bof| &bof.v)
	}
	

	/// negative or out of bounds o values will hit the first and last thing respectively. returns None if tree is empty.
	pub fn remove_at(&mut self, o:N)-> Option<T> {
		self.get_slot_mut(o).map(|b|b.remove())
	}
	
	/// negative or out of bounds o values will get first and last thing respectively. returns None if tree is empty.
	// considering deleting:
	// fn branch_at_index(&self, mut i:usize)-> Option<&Branch<T>> {
	// 	match self.head_node {
	// 		Some(ref cc)=>
	// 			if i >= cc.count as usize {
	// 				None
	// 			}else{
	// 				let mut c = &**cc;
	// 				loop{
	// 					let lnc = count(&c.left) as usize;
	// 					if i < lnc {
	// 						c = &**seriously_unwrap(c.left.as_ref());
	// 					}else if i == lnc {
	// 						break;
	// 					}else{
	// 						i -= lnc+1usize;
	// 						c = &**seriously_unwrap(c.right.as_ref());
	// 					}
	// 				}
	// 				Some(c)
	// 			},
	// 		None=> None
	// 	}
	// }
	
	pub fn clear(&mut self){ self.head_node = None; }
	
	/// Iterates over the buckets
	pub fn slot_iter<'a>(&'a self)-> JostleTreeIter<'a,N, T> {
		JostleTreeIter{c:self.head_node.as_ref().map( |n| leftmost_child(n) )}
	}
}

impl<N:Numeric + Display, T:Display> Display for JostleTree<N,T> {
	fn fmt(&self, f:&mut Formatter)-> Result<(), fmt::Error> {
		try!(write!(f, "JostleTree{{ "));
		for b in self.slot_iter() {
			try!(write!(f, "{}:{} ", b.span, &b.v));
		}
		write!(f, "}}")
	}
}


//Todo: I could make this a bit more efficient than repeated insertion
impl<N, T> std::iter::FromIterator<(N, T)> for JostleTree<N, T> where N:Numeric {
	fn from_iter<I: IntoIterator<Item=(N, T)>>(iter: I) -> Self {
		let mut ret = JostleTree::new();
		for (n, v) in iter {
			ret.insert_back(n, v);
		}
		ret
	}
}



impl<N:Numeric,T> Branch<N,T> {
	pub fn offset(&self)-> N {
		let mut ret = total_span(&self.left);
		let mut p:*const _ = self.parent;
		let mut pp = self;
		loop{
			if p.is_null() {break;}
			let pr = unsafe{&*p};
			if eq_branch(&pr.right, pp) {
				ret = ret + pr.span.clone() + total_span(&pr.left);
			}
			pp = pr;
			p = unsafe{&*pr.parent};
		}
		ret
	}
	fn balance_score(&self) -> isize { deepness(&self.right) as isize - deepness(&self.left) as isize }
	fn update_deepness(&mut self){
		self.deepness = max(deepness(&self.left), deepness(&self.right)) + 1;
	}
	pub fn element(&self)-> &T { &self.v }
	pub fn element_mut(&mut self)-> &mut T { &mut self.v }
	fn update_total_span(&mut self){
		self.total_span = self.span.clone() + total_span(&self.left) + total_span(&self.right);
	}
	fn update_count(&mut self){
		self.count = 1 + count(&self.left) + count(&self.right);
	}
	pub fn get_span(&self)-> N { self.span.clone() }
	pub fn set_span(&mut self, nv:N){
		// if nv < 0 { panic!("attempt to set item in a JostleTree to a negative span"); }
		let dif = self.span.clone() - nv.clone();
		self.span = nv.clone();
		let mut p = self;
		loop{
			p.total_span = p.total_span.clone() + dif.clone();
			if p.parent.is_null() {break;}
			p = unsafe{&mut *p.parent};
		}
	}
	pub fn next(&self)-> Option<&Branch<N,T>> {
		match self.right {
			Some(ref n) =>{
				Some(leftmost_child(n))
			}
			None =>{
				//ascend as right as many times as you have to until you can ascend as left, then you're on the correct node
				unsafe{
					let mut upper_maybe = self.parent;
					let next_node:Option<&Branch<N,T>>;
					loop{
						if upper_maybe != null_mut() {
							if eq_branch(&(*upper_maybe).left, self as *const _) {
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
	}
	pub fn next_mut(&mut self)-> Option<&mut Branch<N,T>> { //haha it's just a copy of the last one because I'm not aware of any way of making it generic over mutability, and unsafe casts from immutable to mutable will be prevented from working by the LLVM flags
		match self.right {
			Some(ref mut n) =>{
				Some(leftmost_child_mut(n))
			}
			None =>{
				//ascend as right as many times as you have to until you can ascend as left, then you're on the correct node
				unsafe{
					let mut upper_maybe = self.parent;
					let next_node:Option<&mut Branch<N,T>>;
					loop{
						if upper_maybe != null_mut() {
							if eq_branch(&(*upper_maybe).left, self as *const _) {
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
	}
	// pub fn prev(&self)-> Option<&Slot> {
	// 	let mut c = &*self as *const _;
	// 	let mut p = &*self.parent as &Branch;
	// 	while p.left.eq_end(c) {
	// 		c = p;
	// 		p = p.parent;
	// 		if p.is_null() { return None }
	// 	}
	// 	//p is now a leftward ascent, descend left once, then as left as you can.
	// 	let mut r = &p.left;
	// 	loop {
	// 		r = match *r {
	// 			End(ref box e)=> return Some(&*e),
	// 			Branch(ref box b)=> &b.right,
	// 		};
	// 	}
	// }seriously_unwrap
}

unsafe fn parents_mut<N,T>(rt:*mut Nref<N,T>, br:&mut Branch<N,T>)-> *mut *mut Branch<N,T> {
	let pp = (*br).parent;
	if !pp.is_null() {
		warp_bptr(if eq_branch(&(*pp).left, br) {
			let r = &mut (*pp).left;
			debug_assert!((*r).is_some());
			r
		}else{
			let r = &mut (*pp).right;
			debug_assert!((*r).is_some());
			r
		})
	}else{
		warp_bptr(rt)
	}
}

//an &mut Slot wrapper for mut_itering and for removals.
pub struct SlotHandle<'a,N:'a,T:'a>{head:*mut Nref<N,T>, v:&'a mut Branch<N,T>}
impl<'a,N:Numeric + 'a,T:'a> SlotHandle<'a,N,T>{
	//returns a ref to the ref in the parent to the given branch
	pub fn remove(self)-> T {
		let SlotHandle{head, v} = self;
		let balancing_starts_with;
		let ret;
		match (v.left.is_some(), v.right.is_some()) {
			(false, false)=> {
				unsafe{
					let rr = parents_mut(head,v);
					let box Branch{v,parent, ..} = seriously_unwrap((*warp_nref(rr)).take());
					balancing_starts_with = parent;
					ret = v;
				}
			}
			(true, false)=> {
				let sp:*mut Branch<N,T> = v.parent;
				ret = replace_self_and_return(v, |cherlf|{
					let Branch{v,left:arm, ..} = cherlf;
					(*seriously_unwrap(arm), v)
				});
				v.parent = sp;
				balancing_starts_with = sp;
			}
			(false, true)=> {
				let sp:*mut Branch<N,T> = v.parent;
				ret = replace_self_and_return(v, |cherlf|{
					let Branch{v,right:arm, ..} = cherlf;
					(*seriously_unwrap(arm), v)
				});
				v.parent = sp;
				balancing_starts_with = sp;
			}
			(true, true)=> {
				//find the highest child, remove it, replace v's v with its, modifying the lineage appropriately. Assumes the nref is some.
				let mut cr:&mut Nref<N,T> = &mut v.left;
				loop{
					if seriously_unwrap(cr.as_ref()).right.is_some() {
						cr = unsafe{ &mut (**warp_bptr(cr)).right };
					}else{
						let (span, item, childs_parent) = replace_self_and_return(cr, |crb|{
							let box Branch{mut left, span, v, parent, ..} = seriously_unwrap(crb);
							if let Some(ref mut lbr) = left {
								lbr.parent = parent;
							}
							(left, (span, v, parent))
						});
						v.span = span;
						balancing_starts_with = childs_parent;
						ret = replace(&mut v.v, item);
						break;
					}
				}
			}
		}
		let mut p:*mut Branch<N,T> = balancing_starts_with;
		while !p.is_null() {
			unsafe{
				let rm = &mut *p;
				balance(parents_mut(head, rm));
				p = rm.parent;
			}
		}
		ret
	}
	
	pub fn next(self)-> Option<Self> {
		let SlotHandle{head, v} = self;
		v.next_mut().map( move|hr| SlotHandle{
			head:head,
			v:hr
		})
	}
	// pub fn prev(self)-> Option<SlotHandle> {
	// 	self.v.prev().map(|hr| SlotHandle(
	// 		unsafe{warp_into_mut(hr)} //we took a mutable so we can rightly give one in return, even if Slot::prev doesn't know that.
	// 	))
	// }
	pub fn element_mut(&mut self)-> &mut T { &mut self.v.v }
	pub fn element(&self)-> &T { &self.v.v }
	pub fn get_span(&self)-> N {self.v.span.clone()}
	pub fn set_span(&mut self, v:N){
		self.v.set_span(v);
	}
	pub fn offset(&self)-> N {self.v.offset()}
}
// impl<'a,T> Deref for SlotHandle<'a,T> {
// 	type Target = T;
// 	fn deref(&mut self) -> &mut Self::Target { &mut self.v.v }
// }



pub struct JostleTreeIter<'a, N:'a, T:'a>{c: Option<&'a Branch<N, T>>}
impl<'a, N:'a + Numeric, T:'a> Iterator for JostleTreeIter<'a, N, T>{
	type Item = &'a Branch<N,T>;
	fn next(&mut self)-> Option<Self::Item> {
		let oc = self.c;
		match oc {
			Some(esr)=> self.c = esr.next(),
			None=> {}
		}
		oc
	}
}

//TODO: Activate this once nll is stabilized
// pub struct JostleTreeMutIter<'a, T:'a>{c: Option<&'a mut Branch<T>>}
// impl<'a, T:'a> Iterator for JostleTreeMutIter<'a, T>{
// 	type Item = &'a mut T; //it's not safe to iterate over branches mutably, changing the spans would have to change the structure of the tree
// 	fn next(&mut self)-> Option<Self::Item> {
// 		let oc = self.c;
// 		match oc {
// 			Some(esr)=> self.c = esr.next_mut(),
// 			None=> {}
// 		}
// 		&mut oc.v
// 	}
// }



impl<N:Numeric + Hash,T:Hash> Hash for JostleTree<N,T> {
	fn hash<H:Hasher>(&self, h:&mut H) {
		for br in self.slot_iter() {
			br.get_span().hash(h);
			br.element().hash(h);
		}
	}
}


impl<N:Numeric,T:PartialEq> PartialEq for JostleTree<N,T> {
	fn eq(&self, other:&JostleTree<N,T>)-> bool {
		self.slot_iter().zip(other.slot_iter()).all(|(l,r)| l.element() == r.element() && l.get_span() == r.get_span() )
	}
	fn ne(&self, other:&JostleTree<N,T>)-> bool { ! self.eq(other) }
}

// impl<T:Eq+Ord> Eq for JostleTree<T> {}
	
	
	
	
#[cfg(test)]
mod tests{
	extern crate rand;
	use super::JostleTree;
	use super::Numeric;
	use super::Nref;
	use std::fmt::Display;
	use std::hash::{Hasher, Hash};
	use std::collections::hash_map::DefaultHasher;
	use self::rand::{XorShiftRng, SeedableRng, Rng};
	
	fn is_balanced<N:Numeric,T:Ord>(tree:&JostleTree<N,T>)-> bool{
		fn of_node<N:Numeric,A>(o:&Nref<N,A>)-> bool {
			match *o {
				Some(ref n) =>{
					n.balance_score().abs() <= 1 && of_node(&n.left) && of_node(&n.right)
				}
				_ => {true}
			}
		}
		of_node(&tree.head_node)
	}
	
	fn sequence_equal<N:Numeric,T:Ord+Display>(v:&JostleTree<N,T>, t:&[T])-> bool {
		v.len() == t.len() && v.slot_iter().zip(t.iter()).all(|(a,i)|{ a.v==*i })
	}

	fn simple_insertions()-> JostleTree<usize, char> {
		let mut candidate = JostleTree::<usize, char>::new();
		candidate.insert_back(3, 'a');
		candidate.insert_back(3, 'b');
		candidate.insert_back(3, 'c');
		candidate.insert_back(3, 'd');
		candidate
	}
	
	#[test]
	fn readme_test() {
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
	}
	
	#[test]
	fn basic_tests(){
		let candidate = simple_insertions();
		assert!(is_balanced(&candidate));
		assert!(candidate.len() == 4);
		assert_eq!(candidate.total_span(), 3*4);
	}
	
	#[test]
	fn iteration_and_order(){
		for (sl, i) in simple_insertions().slot_iter().zip(['a', 'b', 'c'].iter()) {
			assert_eq!(&sl.v, i);
		}
	}
	
	fn hash_of<T:Hash>(v:&T)-> u64 {
		let mut s = DefaultHasher::new();
		v.hash(&mut s);
		s.finish()
	}
	
	#[test]
	fn hashes(){
		let c1 = simple_insertions();
		let mut c2 = simple_insertions();
		c2.insert_at(7, 7, 'a');
		assert!(hash_of(&c1) != hash_of(&c2));
		let c3 = simple_insertions();
		assert!(hash_of(&c1) == hash_of(&c3));
	}
	
	#[test]
	fn removal(){
		let mut candidate = simple_insertions();
		{
			let c = candidate.get_slot_mut(4).unwrap();
			c.remove();
		}
		assert!(sequence_equal(&candidate, &['a','c','d']));
	}
	
	fn ease_in_out(v:f32)-> f32 {
		if v < 0.5 { v*v } else {
			let vso = v-1.;
			1. -vso*vso
		}
	}
	
	fn sum_of_spans<T>(n:&Nref<usize, T>)-> usize {
		if let Some(ref n) = *n {
			n.span + sum_of_spans(&n.left) + sum_of_spans(&n.right)
		}else{
			0
		}
	}
	
	#[inline(always)]
	fn lcg1(v:u32)-> u32 { 1664525u32.wrapping_mul(v).wrapping_add(1013904223u32) }
	#[inline(always)]
	fn lcg2(v:u32)-> u32 { 22695477u32.wrapping_mul(v).wrapping_add(1u32) }
	#[inline(always)]
	fn seed_slice(v:u32)-> [u32;4] {
		let r1 = lcg1(v);
		let r2 = lcg1(v.wrapping_add(76));
		let r3 = lcg2(r2);
		let r4 = lcg2(r1);
		[r1,r2,r3,r4]
	}
	
	fn seed_of_the_now()-> [u32;4] {
		let sotn = rand::random();
		println!("using rng seed: {}.", sotn);
		seed_slice(sotn)
	}
	
	#[test]
	fn big_attack(){
		let mut candidate = JostleTree::<usize, u64>::new();
		let mut katy = XorShiftRng::from_seed(seed_of_the_now()); //the seed should be printed if there's a break. use seed_slice(<printed seed>) instead to reproduce those conditions.
		let mut rand_unit = ||{ katy.next_f32() };
		let cycles:usize = 30;
		let cyclesize:usize = 300;
		let input_size:usize = 1000;
		let mut i:usize = 0;
		loop {
			let o = (candidate.total_span() as f32 *rand_unit()).floor() as usize;
			let rspan = 80.*ease_in_out(rand_unit());
			candidate.insert_at(o, (rspan*rspan) as usize, i as u64);
			i += 1;
			if i >= input_size {break}
		}
		// println!("before cycles {:#?}", &candidate);
		for _ in 0..cycles {
			for _ in 0..cyclesize {
				let pos = (candidate.total_span() as f32 *rand_unit()).floor() as usize;
				candidate.remove_at(pos);
			}
			// println!("before insertions {:#?}", &candidate);
			for _ in 0..cyclesize {
				let o = candidate.total_span() as f32 *rand_unit();
				let rspan = 80. *ease_in_out(rand_unit());
				candidate.insert_at((o*rspan*rspan).floor() as usize, 0, i as u64);
				i += 1;
			}
			// println!("after insertions {:#?}", &candidate);
		}
		
		assert_eq!(candidate.len(), input_size as usize);
		assert!(is_balanced(&candidate));
		assert_eq!(sum_of_spans(&candidate.head_node), candidate.total_span());
	}
}