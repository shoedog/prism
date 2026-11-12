struct T { h: i32 }
struct S { f: i32 }
struct Holder { p: Box<S> }
struct Outer { g: Box<T> }

trait Exchange {
    fn exchange(&mut self, other: &mut Self);
}
impl Exchange for Box<S> {
    fn exchange(&mut self, other: &mut Self) {
        std::mem::swap(self, other);
    }
}

fn deref_restore() {
    let mut s = Box::new(Box::new(S { f: 0 }));
    s.f = 1; // @deref_restore:s.f:D
    let saved = std::mem::replace(&mut *s, Box::new(S { f: 0 }));
    s.f = 2;
    *s = saved;
    std::hint::black_box(s.f); // @deref_restore:s.f:U
}

fn prefix_restore() {
    let mut s = Holder { p: Box::new(S { f: 0 }) };
    s.p.f = 1; // @prefix_restore:s.p.f:D
    let saved = s.p;
    s.p = Box::new(S { f: 0 });
    s.p.f = 2;
    s.p = saved;
    std::hint::black_box(s.p.f); // @prefix_restore:s.p.f:U
}

fn auto_ref() {
    let mut s = Box::new(S { f: 0 });
    let mut other = Box::new(S { f: 0 });
    s.f = 1; // @auto_ref:s.f:D
    s.exchange(&mut other);
    s.f = 2;
    s.exchange(&mut other);
    std::hint::black_box(s.f); // @auto_ref:s.f:U
}

fn replace_prefix(mut s: Outer) -> i32 {
    s.g.h = 1; // @replace_prefix:s.g.h:D
    let saved = std::mem::replace(&mut s.g, Box::new(T { h: 0 }));
    s.g.h = 2;
    s.g = saved;
    s.g.h // @replace_prefix:s.g.h:U
}

fn ref_mut_ref<'a>(s: &mut &'a mut S, b: &'a mut S) -> i32 {
    s.f = 1; // @ref_mut_ref:s.f:D
    let old = std::mem::replace(s, b);
    s.f = 2;
    *s = old;
    s.f // @ref_mut_ref:s.f:U
}

fn macro_mut_field(a: Box<S>, b: Box<S>) -> i32 {
    let mut s = a;
    let mut t = b;
    s.f = 1; // @macro_mut_field:s.f:D
    dbg!(std::mem::swap(&mut s, &mut t));
    s.f = 2;
    dbg!(std::mem::swap(&mut s, &mut t));
    s.f // @macro_mut_field:s.f:U
}

fn ref_mut_field(a: Box<S>, b: Box<S>) -> i32 {
    let mut s = a;
    let mut t = b;
    s.f = 1; // @ref_mut_field:s.f:D
    { let ref mut r = s; std::mem::swap(r, &mut t); }
    s.f = 2;
    { let ref mut r = s; std::mem::swap(r, &mut t); }
    s.f // @ref_mut_field:s.f:U
}

fn field_control(s: &mut S) {
    s.f = 1; // @field_control:s.f:D
    s.f = 2;
    std::hint::black_box(s.f); // @field_control:s.f:U
}

fn local_macro_u() -> i32 {
    let mut x = 1; // @local_macro_u:x:D
    macro_rules! bump { () => { x + 1 }; }
    x = bump!(); // @local_macro_u:x:U
    x
}

macro_rules! install_pick {
    ($captured:ident) => {
        macro_rules! pick {
            (x) => { $captured };
        }
    };
}

fn generated_hygiene() -> i32 {
    let x = 1; // @generated_hygiene:x:D
    install_pick!(x);
    let x = 2;
    pick!(x) // @generated_hygiene:x:U
}

fn plain_use_control() {
    let x = 1;
    let mut x = x; // @plain_use_control:x:D
    x = 2;
    std::hint::black_box(x); // @plain_use_control:x:U
}

fn swap_around() {
    let mut x = 1; // @swap_around:x:D
    let mut y = 0;
    std::mem::swap(&mut x, &mut y);
    x = 2;
    std::mem::swap(&mut x, &mut y);
    std::hint::black_box(x); // @swap_around:x:U
}

fn swap_before() {
    let mut x = 1; // @swap_before:x:D
    let mut y = 0;
    std::mem::swap(&mut x, &mut y);
    x = 2;
    std::hint::black_box(x); // @swap_before:x:U
}

fn swap_after() {
    let mut x = 1; // @swap_after:x:D
    let mut y = 0;
    x = 2;
    std::mem::swap(&mut x, &mut y);
    std::hint::black_box(x); // @swap_after:x:U
}

fn swap_in_dbg() {
    let mut x = 1; // @swap_in_dbg:x:D
    let mut y = 0;
    dbg!(std::mem::swap(&mut x, &mut y));
    x = 2;
    dbg!(std::mem::swap(&mut x, &mut y));
    std::hint::black_box(x); // @swap_in_dbg:x:U
}

fn ref_mut_ident() {
    let mut x = 1; // @ref_mut_ident:x:D
    let ref mut r = x;
    *r = 5;
    x = 2;
    std::hint::black_box(x); // @ref_mut_ident:x:U
}

fn raw_mut_ident() {
    let mut x = 1; // @raw_mut_ident:x:D
    let p = &raw mut x;
    x = 2;
    std::hint::black_box((x, p)); // @raw_mut_ident:x:U
}

fn unborrowed_control() {
    let mut x = 1; // @unborrowed_control:x:D
    x = 2;
    std::hint::black_box(x); // @unborrowed_control:x:U
}
