fn no_value(c: bool) {
    let mut x = 1;
    if c { x = 2; }
    let x;
    sink(x);
}

fn tuple_pat(c: bool) {
    let mut x = 1;
    if c { x = 2; }
    let (x, y) = pair();
    sink(x);
}

fn if_let(c: bool, o: Option<i32>) {
    let mut x = 1;
    if c { x = 2; }
    if let Some(x) = o {
        sink(x);
    }
    sink(x);
}

fn while_let(c: bool, mut it: I) {
    let mut x = 1;
    if c { x = 2; }
    while let Some(x) = it.next() {
        sink(x);
    }
    sink(x);
}

fn match_binding(c: bool, o: Option<i32>) {
    let mut x = 1;
    if c { x = 2; }
    match o {
        Some(x) => sink(x),
        None => {}
    }
    sink(x);
}

fn for_target(c: bool, xs: Vec<i32>) {
    let mut x = 1;
    if c { x = 2; }
    for x in xs {
        sink(x);
    }
    sink(x);
}

fn closure_param(c: bool) {
    let mut x = 1;
    if c { x = 2; }
    let f = |x: i32| sink(x);
    f(2);
    sink(x);
}

fn let_else(c: bool, o: Option<i32>) {
    let mut x = 1;
    if c { x = 2; }
    let Some(x) = o else { return };
    sink(x);
}

fn nested_assign(c: bool) {
    let mut x = 1;
    if c { x = 2; }
    let mut y = 0;
    y = (x = 3);
    sink(x);
}

fn macro_write(c: bool) {
    let mut x = 1;
    if c { x = 2; }
    m!(x = 3);
    sink(x);
}

fn attributed(c: bool) {
    let mut x = 1;
    if c { x = 2; }
    #[cfg(feature = "f")]
    let x = 5;
    sink(x);
}

fn attributed_assign(c: bool) {
    let mut x = 1;
    if c { x = 2; }
    #[cfg(feature = "f")]
    x = 5;
    sink(x);
}

fn closure_writer() {
    let mut x = 0;
    let mut c = || x = 9;
    c();
    x = 2;
    sink(x);
}

fn closure_use() {
    let mut x = 1;
    x = 2;
    let c = || sink(x);
    c();
}

fn macro_read() {
    let mut x = source();
    x = clean();
    println!("{}", x);
}

fn nested_fn() {
    let mut x = 1;
    fn inner() {
        let mut x = 5;
        x = 6;
        sink(x);
    }
    x = 2;
    sink(x);
}

fn deref_write(x: &mut i32) {
    *x = 1;
    sink(x);
}

fn async_block() {
    let mut x = 0;
    let fut = async { x = 5; };
    x = 2;
    sink(x);
}

fn two_occurrences() {
    let mut x = 1;
    x = 2;
    sink(x, x);
}

fn defs_between() {
    let mut x = 0;
    x = 1;
    foo(if c { x = 5; 1 } else { 2 });
    sink(x);
}

fn while_loop_reentry(mut n: i32) {
    let mut x = 0;
    while n > 0 {
        sink(x);
        x = 1;
        n -= 1;
    }
}
