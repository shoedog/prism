fn repoint(a: &mut S, b: &mut S) {
    let mut s = &mut *a;
    s.f = 1;
    s = &mut *b;
    s.f = 2;
    s = &mut *a;
    sink(s.f);
}

fn box_move() {
    let mut s = Box::new(S { f: 0 });
    s.f = 1;
    let saved = s;
    s = Box::new(S { f: 0 });
    s.f = 2;
    s = saved;
    sink(s.f);
}

fn if_let_rebind(a: &mut S) {
    let s = &mut *a;
    s.f = 1;
    s.f = 2;
    if let Some(s) = other() {
        sink(s.f);
    }
}

fn second_let() {
    let mut s = make();
    s.f = 1;
    let mut s = make2();
    s.f = 2;
    sink(s.f);
}

fn closure_param(s: &mut S) {
    s.f = 1;
    s.f = 2;
    let g = |s: &mut S| s.f;
    sink(s.f);
}

fn stable_let(x: &mut S) {
    let s = &mut *x;
    s.f = 1;
    s.f = 2;
    sink(s.f);
}

fn stable_formal(s: &mut S) {
    s.f = 1;
    s.f = 2;
    sink(s.f);
}

fn swap_base(a: &mut S, mut t: &mut S) {
    let mut s = &mut *a;
    s.f = 1;
    s.f = 2;
    std::mem::swap(&mut s, &mut t);
    sink(s.f);
}

impl S {
    fn method(&mut self) {
        self.f = 1;
        self.f = 2;
        sink(self.f);
    }
}

fn hygiene() -> i32 {
    let x = 1;
    macro_rules! pick { ($u:ident) => { x }; }
    let x = 2;
    pick!(x)
}

fn println_use() {
    let mut x = 1;
    x = 2;
    println!("{}", x);
}

fn macro_u() {
    macro_rules! m { ($v:ident) => { $v }; }
    let mut x = 0;
    x = 1; m!(x);
    sink(x);
}

fn block_expr() {
    let mut x = 0;
    let y = {
        x = 1;
        x
    };
    sink(x);
    sink(y);
}

fn tail_in_block() {
    let mut x = 0;
    x = 1;
    { x = 2 }
    sink(x);
}

fn macro_let() {
    let mut x = 0;
    x = 1;
    shadow!(x);
    sink(x);
}

fn nested_fn_owner() {
    fn inner() {
        let mut y = 1;
        y = 2;
        sink(y);
    }
    inner();
}
