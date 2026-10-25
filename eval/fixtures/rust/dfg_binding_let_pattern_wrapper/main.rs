struct W(i32);

fn wrapper(w: W, z: W) {
    let W(a) = w;
    let W(b) = z;
    sink(a);
    sink(b);
}

fn option(o: Option<i32>, p: Option<i32>) {
    let Some(v) = o;
    let Some(w) = p;
    sink(w);
}
