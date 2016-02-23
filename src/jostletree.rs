use std::hash::{Hash, Hasher};
use std::mem::{uninitialized, forget, transmute, transmute_copy, replace};
use std::ptr::{copy_nonoverlapping, null_mut, read};
use std::fmt;
use std::fmt::{Display, Formatter};
use std::cmp::{max};
use std::iter::Iterator;


//Some of this code is uglier than perhaps it could be given today's Rust. Unfortunately it was written with yesterday's rust, which couldn't end borrows before the end of the borrowing match block, for instance. For data structure code, some ugliness is required, in these parts.



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

//serious functions panic on failure in debug mode and have undefined behavior on failure in release mode(but're perfectly efficient(presumably))
#[inline(always)]
pub fn seriously_unreachable()-> ! {
	seriously_unreach("this code is not supposed to be reachable") }
#[inline(always)]
pub fn seriously_unreach(s:&str)-> ! {
	if cfg!(debug_assertions) { panic!("{}", s) }
	else{ unsafe{::std::intrinsics::unreachable()} }
}
#[inline(always)]
pub fn seriously_unwrap<T>(v:Option<T>)-> T {
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


unsafe fn warp_nref<'a, T>(v:*mut *mut Branch<T>)-> &'a mut Nref<T> { transmute(v) }
unsafe fn warp_bptr<T>(v:*mut Nref<T>)-> *mut *mut Branch<T> { transmute(v) }
unsafe fn warp_bptr_const<T>(v:*const Nref<T>)-> *const *const Branch<T> { transmute(v) }
unsafe fn warp_nullable<T>(v:*mut Nref<T>)-> *mut Branch<T> { transmute_copy(&*v) }


unsafe fn rotate_right<A>(old_root: *mut *mut Branch<A>){ //assumes root and leftofroot are some.
	let orp = *old_root;
	let rleft:*mut Nref<A> = &mut (*orp).left;
	let rleftright:*mut Nref<A> = &mut seriously_unwrap((*rleft).as_mut()).right;
	let ol_widdly = warp_nullable(rleftright);
	let root_to_be:*mut Branch<A> = &mut **seriously_unwrap((*rleft).as_mut());
	let old_root_as_nref:*mut Nref<A> = warp_nref(old_root);
	if !ol_widdly.is_null() {
		cp_nonoverlapping(old_root as *const _, &mut (*ol_widdly).parent);
	}
	cp_nonoverlapping(& (*orp).parent, &mut (*root_to_be).parent);
	cp_nonoverlapping(& root_to_be, &mut (*orp).parent);
	switch_left(old_root_as_nref, rleft, rleftright);
	let old_span = (*orp).total_span;
	let rising_span = (*root_to_be).total_span;
	let widly_span = if ol_widdly.is_null() { 0 }else{ (*ol_widdly).total_span };
	(*root_to_be).total_span = old_span;
	(*orp).total_span = old_span - (rising_span - widly_span);
	(*orp).update_deepness();
	(*orp).update_count();
	(*root_to_be).update_deepness();
	(*root_to_be).update_count();
}

unsafe fn rotate_left<A>(old_root: *mut *mut Branch<A>){ //assumes root and rightofroot are some.
	let orp = *old_root;
	let rright:*mut Nref<A> = &mut (*orp).right;
	let rrightleft:*mut Nref<A> = &mut seriously_unwrap((*rright).as_mut()).left;
	let ol_widdly = warp_nullable(rrightleft);
	let root_to_be:*mut Branch<A> = &mut **seriously_unwrap((*rright).as_mut());
	let old_root_as_nref:*mut Nref<A> = warp_nref(old_root);
	if !ol_widdly.is_null() {
		cp_nonoverlapping(old_root as *const _, &mut (*ol_widdly).parent);
	}
	cp_nonoverlapping(& (*orp).parent, &mut (*root_to_be).parent);
	cp_nonoverlapping(&root_to_be, &mut (*orp).parent);
	switch_left(old_root_as_nref, rright, rrightleft);
	let old_span = (*orp).total_span;
	let rising_span = (*root_to_be).total_span;
	let widly_span = if ol_widdly.is_null() { 0 }else{ (*ol_widdly).total_span };
	(*root_to_be).total_span = old_span;
	(*orp).total_span = old_span - (rising_span - widly_span);
	(*orp).update_deepness();
	(*orp).update_count();
	(*root_to_be).update_deepness();
	(*root_to_be).update_count();
}

#[derive(Debug)]
pub struct Branch<T> {
	pub v:T,
	deepness:u8,
	count:u32,
	span:u32,
	total_span:u32,
	parent: *mut Branch<T>,
	left: Nref<T>,
	right: Nref<T>,
}
type Nref<T> = Option<Box<Branch<T>>>;
fn eq_branch<T>(v:&Nref<T>, other:*const Branch<T>)-> bool { (other == unsafe{*warp_bptr_const(v)}) }

fn deepness<T>(nref:&Nref<T>)-> u8 {
	match *nref { Some(ref bbs) => bbs.deepness, None => 0 } }
fn count<T>(nref:&Nref<T>)-> u32 {
	match *nref { Some(ref bbs)=> bbs.count, None=> 0 } }
fn total_span<T>(nref:&Nref<T>)-> u32 {
	match *nref { Some(ref bbs)=> bbs.total_span, None=> 0 } }

unsafe fn balance<T>(rootofrotation: *mut *mut Branch<T>){
	//OPTIMIZING: Might not always be necessary to update all three scores each time balance is called
	let ro:&mut Branch<T> = &mut **rootofrotation;
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

fn leftmost_child<T>(n: &Branch<T>)-> &Branch<T> {
	match n.left {
		Some(ref nl)=> leftmost_child(nl),
		None=> n
	}
}
fn leftmost_child_mut<T>(n: &mut Branch<T>)-> &mut Branch<T> {
	match n.left {
		Some(ref mut nl)=> leftmost_child_mut(nl),
		None=> n
	}
}

// fn leftmost_child_mut<T>(n: &mut Branch<T>)-> &mut Branch<T> {
// 	unsafe{warp_into_mut(leftmost_child(n))}
// }

#[derive(Debug)]
pub struct JostleTree<T>{
	head_node: Nref<T>,
}
impl<T> JostleTree<T> {
	pub fn new()-> JostleTree<T> { JostleTree{head_node:None} }
	pub fn is_empty(&self)-> bool { self.head_node.is_none() }
	pub fn len(&self)-> usize { count(&self.head_node) as usize }
	pub fn total_span(&self)-> u32 { total_span(&self.head_node) }
	///inserts element v at or before whatever is at_offset
	pub fn insert<'a>(&'a mut self, v:T, span:u32, at_offset:u32)-> SlotHandle<'a,T>{
		let r;
		{
			let mut parent = null_mut();
			let mut cn:&mut Nref<T> = &mut self.head_node;
			let mut offset = at_offset;
			// println!("inserting {}", &v);
			loop{
				let fcn:*mut _ = match *cn {
					Some(ref mut n)=> {
						// println!("looking at {}", &n.v);
						let bspan = total_span(&n.left) + n.span;
						n.total_span += span;
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
						// if !parent.is_null() { println!("parent is {}", unsafe{&(*parent).v}); }
						let mut bb = box Branch{v:v, span:span, deepness:1, count:1, total_span:span, parent:parent, left:None, right:None};
						r = unsafe{warp_lifetime_mut(&mut *bb)}; //we know that the location of Branch doesn't change, it doesn't get freed, and that it lives as long as the return value.
						*rcn = Some(bb);
						break;
					}
				};
				cn = unsafe{warp_ptr_into_mut(fcn)};
			}
		};
		unsafe{
			let mut p = r.parent;
			while !p.is_null() {
				// println!("{}", &(*p).v);
				balance(parents_mut(&mut self.head_node, &mut*p));
				p = (*p).parent;
			}
		}
		SlotHandle{head:&mut self.head_node, v:r}
	}
	
	
	//good version: Does not work because the borrowck is an IMBICILE, so I'll keep this around for later
	// pub fn branch_at_offset(&mut self, mut o:u32)-> &mut Branch<T> { //negative or out of bounds o values will get first and last thing respectively
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
	
	//see above for the reasonable version
	pub fn branch_at_offset(&mut self, mut o:u32)-> Option<&mut Branch<T>> { //negative or out of bounds o values will get first and last thing respectively. returns None if tree is empty.
		unsafe{//alright, you know what, rust? I'm going to use a fucking pointer for this until you figure out how to infer basic limits to borrows, because I've tried everything that makes sense and none of it is enough for you.
			let mut c:*mut Branch<T> = match self.head_node {
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
					let rts = lts + (*c).span;
					if o < rts { return Some(&mut *c) }
					else {
						c = match (*c).right {
							Some(ref mut rb)=> &mut **rb,
							None=> return Some(&mut *c),
						};
						o -= rts;
					}
				}
			}
		}
	}
	
	pub fn remove_at(&mut self, o:u32)-> Option<T> {
		self.slot_at_offset(o).map(|b|b.remove())
	}
	///returns none iff index out of bounds
	pub fn branch_at_index(&self, mut i:usize)-> Option<&Branch<T>> {
		match self.head_node {
			Some(ref cc)=>
				if i >= cc.count as usize {
					None
				}else{
					let mut c = &**cc;
					loop{
						let lnc = count(&c.left) as usize;
						if i < lnc {
							c = &**seriously_unwrap(c.left.as_ref());
						}else if i == lnc {
							break;
						}else{
							i -= lnc+1usize;
							c = &**seriously_unwrap(c.right.as_ref());
						}
					}
					Some(c)
				},
			None=> None
		}
	}
	// pub fn branch_at_index_mut(&mut self, i:usize)-> Option<&mut Branch<T>> {
	// 	self.branch_at_index(i).map(|rm| unsafe{warp_into_mut(rm)})
	// }
	pub fn slot_at_offset<'a>(&'a mut self, o:u32)-> Option<SlotHandle<'a,T>> { //None if tree is empty
		let head_ptr:*mut _ = &mut self.head_node;
		self.branch_at_offset(o).map(move|bof| SlotHandle{head:head_ptr, v:bof})
	}
	pub fn clear(&mut self){
		self.head_node = None;
	}
	pub fn iter<'a>(&'a self)-> JostleTreeIter<'a, T> {
		JostleTreeIter{c:self.head_node.as_ref().map( |n|leftmost_child(n) )}
	}
}

impl<T:Display> Display for JostleTree<T> {
	fn fmt(&self, f:&mut Formatter)-> Result<(), fmt::Error> {
		try!(write!(f, "JostleTree{{ "));
		for b in self.iter() {
			try!(write!(f, "{}:{} ", b.span, &b.v));
		}
		write!(f, "}}")
	}
}


impl<T> Branch<T>{
	pub fn offset(&self)-> u32 {
		let mut ret = total_span(&self.left);
		let mut p:*const _ = self.parent;
		let mut pp = self;
		loop{
			if p.is_null() {break;}
			let pr = unsafe{&*p};
			if eq_branch(&pr.right, pp) {
				ret += pr.span + total_span(&pr.left);
			}
			pp = pr;
			p = unsafe{&*pr.parent};
		}
		ret
	}
	fn balance_score(&self) -> i32 { deepness(&self.right) as i32 - deepness(&self.left) as i32 }
	fn update_deepness(&mut self){
		self.deepness = max(deepness(&self.left), deepness(&self.right)) + 1;
	}
	pub fn element(&self)-> &T { &self.v }
	pub fn element_mut(&mut self)-> &mut T { &mut self.v }
	fn update_total_span(&mut self){
		self.total_span = self.span + total_span(&self.left) + total_span(&self.right);
	}
	fn update_count(&mut self){
		self.count = 1 + count(&self.left) + count(&self.right);
	}
	pub fn get_span(&self)-> u32 { self.span }
	pub fn set_span(&mut self, nv:u32){
		// if nv < 0 { panic!("attempt to set item in a JostleTree to a negative span"); }
		let dif = self.span - nv;
		self.span = nv;
		let mut p = self;
		loop{
			p.total_span += dif;
			if p.parent.is_null() {break;}
			p = unsafe{&mut *p.parent};
		}
	}
	pub fn next(&self)-> Option<&Branch<T>> {
		match self.right {
			Some(ref n) =>{
				Some(leftmost_child(n))
			}
			None =>{
				//ascend as right as many times as you have to until you can ascend as left, then you're on the correct node
				unsafe{
					let mut upper_maybe = self.parent;
					let next_node:Option<&Branch<T>>;
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
	pub fn next_mut(&mut self)-> Option<&mut Branch<T>> { //haha it's just a copy of the last one because no mutable generics and unsafe casts from immutable to mutable will be prevented from working by the LLVM flags
		match self.right {
			Some(ref mut n) =>{
				Some(leftmost_child_mut(n))
			}
			None =>{
				//ascend as right as many times as you have to until you can ascend as left, then you're on the correct node
				unsafe{
					let mut upper_maybe = self.parent;
					let next_node:Option<&mut Branch<T>>;
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
	// }
}

unsafe fn parents_mut<T>(rt:*mut Nref<T>, br:&mut Branch<T>)-> *mut *mut Branch<T> {
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
pub struct SlotHandle<'a,T:'a>{head:*mut Nref<T>, v:&'a mut Branch<T>}
impl<'a,T> SlotHandle<'a,T>{
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
				let sp:*mut Branch<T> = v.parent;
				ret = replace_self_and_return(v, |cherlf|{
					let Branch{v,left:arm, ..} = cherlf;
					(*seriously_unwrap(arm), v)
				});
				v.parent = sp;
				balancing_starts_with = sp;
			}
			(false, true)=> {
				let sp:*mut Branch<T> = v.parent;
				ret = replace_self_and_return(v, |cherlf|{
					let Branch{v,right:arm, ..} = cherlf;
					(*seriously_unwrap(arm), v)
				});
				v.parent = sp;
				balancing_starts_with = sp;
			}
			(true, true)=> {
				//find the highest child, remove it, replace v's v with its, modifying the lineage appropriately. Assumes the nref is some.
				let mut cr:&mut Nref<T> = &mut v.left;
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
		let mut p:*mut Branch<T> = balancing_starts_with;
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
	pub fn get_span(&self)-> u32 {self.v.span}
	pub fn set_span(&mut self, v:u32){
		self.v.set_span(v);
	}
	pub fn offset(&self)-> u32 {self.v.offset()}
}
// impl<'a,T> Deref for SlotHandle<'a,T> {
// 	type Target = T;
// 	fn deref(&mut self) -> &mut Self::Target { &mut self.v.v }
// }


pub struct JostleTreeIter<'a, T:'a>{c: Option<&'a Branch<T>>}
impl<'a, T:'a> Iterator for JostleTreeIter<'a, T>{
	type Item = &'a Branch<T>;
	fn next(&mut self)-> Option<Self::Item> {
		let oc = self.c;
		match oc {
			Some(esr)=> self.c = esr.next(),
			None=> {}
		}
		oc
	}
}

impl<T:Hash> Hash for JostleTree<T> {
	fn hash<H:Hasher>(&self, h:&mut H) {
		for br in self.iter() {
			h.write_u32(br.get_span());
			br.element().hash(h);
		}
	}
}


impl<T:PartialEq> PartialEq for JostleTree<T> {
	fn eq(&self, other:&JostleTree<T>)-> bool {
		self.iter().zip(other.iter()).all(|(l,r)| l.element() == r.element() && l.get_span() == r.get_span() )
	}
	fn ne(&self, other:&JostleTree<T>)-> bool { ! self.eq(other) }
}

// impl<T:Eq+Ord> Eq for JostleTree<T> {}
	
	
	
	
#[cfg(test)]
mod tests{
	extern crate rand;
	use super::JostleTree;
	use super::Nref;
	use std::fmt::Display;
	use std::hash::{SipHasher, Hasher, Hash};
	use self::rand::{XorShiftRng, SeedableRng, Rng};
	
	#[inline(always)]
	fn fairly_eq(a:u32, b:u32)-> bool {
		a == b
	}
	
	fn is_balanced<T:Ord>(tree:&JostleTree<T>)-> bool{
		fn of_node<A>(o:&Nref<A>)-> bool{
			match *o {
				Some(ref n) =>{
					n.balance_score().abs() <= 1 && of_node(&n.left) && of_node(&n.right)
				}
				_ => {true}
			}
		}
		of_node(&tree.head_node)
	}
	
	fn sequence_equal<T:Ord+Display>(v:&JostleTree<T>, t:&[T])-> bool {
		v.len() == t.len() && v.iter().zip(t.iter()).all(|(a,i)|{ a.v==*i })
	}

	fn simple_insertions()-> JostleTree<u64> {
		let mut candidate = JostleTree::<u64>::new();
		candidate.insert(4, 3, 0);
		candidate.insert(3, 3, 0);
		candidate.insert(2, 3, 0);
		candidate.insert(1, 3, 0);
		candidate
	}
	
	#[test]
	fn basic_tests(){
		let candidate = simple_insertions();
		assert!(is_balanced(&candidate));
		assert!(candidate.len() == 4);
		assert!(fairly_eq(candidate.total_span(), 3*4));
	}
	
	#[test]
	fn iteration_and_order(){
		for (sl, i) in simple_insertions().iter().zip(1..) {
			assert_eq!(sl.v, i);
			assert!(fairly_eq(sl.offset(), ((i-1) as u32)*3));
		}
	}
	
	fn hash_of<T:Hash>(v:&T)-> u64 {
		let mut s = SipHasher::new();
		v.hash(&mut s);
		s.finish()
	}
	
	#[test]
	fn hashes(){
		let c1 = simple_insertions();
		let mut c2 = simple_insertions();
		c2.insert(0, 7, 7);
		assert!(hash_of(&c1) != hash_of(&c2));
		let c3 = simple_insertions();
		assert!(hash_of(&c1) == hash_of(&c3));
	}
	
	#[test]
	fn removal(){
		let mut candidate = simple_insertions();
		{
			let c = candidate.slot_at_offset(4).unwrap();
			c.remove();
		}
		assert!(sequence_equal(&candidate, &[1,3,4]));
	}
	
	fn ease_in_out(v:f32)-> f32 {
		if v < 0.5 { v*v } else {
			let vso = v-1.;
			1. -vso*vso
		}
	}
	
	fn sum_of_spans<T>(n:&Nref<T>)-> u32 {
		match *n {
			Some(ref n)=> n.span + sum_of_spans(&n.left) + sum_of_spans(&n.right),
			None=> 0,
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
	
	#[test]
	fn big_attack(){
		let mut candidate = JostleTree::<u64>::new();
		let mut katy = XorShiftRng::from_seed(seed_slice(4)); //change this to try with a different starting set. (When you're testing, you want reproducible conditions, so a true RNG probably is not a good idea)
		let mut rand_unit = ||{ katy.next_f32() };
		let cycles:usize = 30;
		let cyclesize:usize = 300;
		let input_size:usize = 1000;
		let mut i:usize = 0;
		loop {
			let o:u32 = (candidate.total_span() as f32 *rand_unit()).floor() as u32;
			let rspan = 80.*ease_in_out(rand_unit());
			candidate.insert(i as u64, (rspan*rspan) as u32, o);
			i += 1;
			if i >= input_size {break}
		}
		// println!("before cycles {:#?}", &candidate);
		for _ in 0..cycles {
			for _ in 0..cyclesize {
				let pos = (candidate.total_span() as f32 *rand_unit()).floor() as u32;
				candidate.remove_at(pos);
			}
			// println!("before insertions {:#?}", &candidate);
			for _ in 0..cyclesize {
				let o = (candidate.total_span() as f32 *rand_unit()) as u32;
				let rspan = 80. *ease_in_out(rand_unit());
				candidate.insert(i as u64, (rspan*rspan).floor() as u32, o);
				i += 1;
			}
			// println!("after insertions {:#?}", &candidate);
		}
		
		assert_eq!(candidate.len(), input_size as usize);
		assert!(is_balanced(&candidate));
		assert_eq!(sum_of_spans(&candidate.head_node), candidate.total_span());
	}
}