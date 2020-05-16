// Actual utilitary functions that go nowhere else,
// and aren't syntax extensions (see slang.rs)

// Given two Vecs, returns their set difference as two Vecs 
pub fn set_diff<T :Ord> (mut left :Vec<T>, mut right :Vec<T>) -> (Vec<T>, Vec<T>) {
    use std::cmp::Ordering::*;
    
    if left.is_empty() || right.is_empty() {
        return (left, right)
    }
    
    left.sort();
    right.sort();
    // Iterators and values
    let mut l = left.into_iter();
    let mut r = right.into_iter();
    let mut a = None::<T>;
    let mut b = None::<T>;
    
    let mut nleft = Vec::<T>::new();
    let mut nright = Vec::<T>::new();
    // Util function
    let mut push_l = |a :Option<T>| { nleft.push(a.unwrap()) };
    let mut push_r = |b :Option<T>| { nright.push(b.unwrap()) };
    
    loop {
        if a.is_none() {
            a = l.next();
        }
        if b.is_none() {
            b = r.next();
        }
        
        match (&a, &b) {
            (None, None) => break,
            (Some(_), None) => push_l(a.take()),
            (None, Some(_)) => push_r(b.take()),
            (Some(v1), Some(v2)) => match v1.cmp(v2) {
                Less => push_l(a.take()),
                Greater => push_r(b.take()),
                Equal => { a.take(); b.take(); },
            },
        }
    }
    (nleft, nright)
}

// Single quotes a string for the shell
pub fn shell_escape (s :&str) -> String {
	format!("'{}'", s.replace("'", r"'\''"))
}