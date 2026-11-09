fn chain() {
    let mut x = source();
    x = clean();
    sink(x);
}

fn shadow() {
    let x = 1;
    let x = x + 1;
    sink(x);
}

fn u_rows() {
    let mut x = 0;
    x = 1;
    x += 1;
    x = x + 1;
    sink(x);
}

fn u_other_writes(v: &mut Vec<i32>, s: &mut S, mut p: i32) {
    let mut x = 0;
    v[0] = 1;
    s.f = 1;
    let r = &mut p;
    *r = 1;
    x = 2;
    sink(x);
}

fn same_line() {
    let mut x = 1; x = 2;
    sink(x);
}

fn multi_line() {
    let mut x = source();
    x =
        clean();
    sink(
        x,
    );
}

fn field_paths(mut s: S) {
    s.f = 1;
    s.f = 2;
    sink(s.f);
    s.g = 3;
    s = other();
    sink(s.g);
}

fn param_dominated(mut p: i32) {
    p = 5;
    sink(p);
}

fn try_dominates() -> Result<(), E> {
    let mut x = 1;
    x = f()?;
    sink(x);
    Ok(())
}
