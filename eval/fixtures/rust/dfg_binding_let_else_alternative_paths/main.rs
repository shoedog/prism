struct S {
    f: i32,
}

fn field(o: Option<i32>) {
    let mut s = S { f: source() };
    let Some(v) = o else {
        s.f = clean();
        return;
    };
    sink(s.f);
}

fn deref(o: Option<i32>, r: &mut i32) {
    *r = source();
    let Some(v) = o else {
        *r = clean();
        return;
    };
    sink(*r);
}

fn compound_deref(o: Option<i32>, r: &mut i32) {
    *r = source();
    let Some(v) = o else {
        *r += 1;
        return;
    };
    sink(*r);
}

fn index(o: Option<i32>) {
    let mut a = source_vec();
    let Some(v) = o else {
        a[0] = clean();
        return;
    };
    sink(a);
}

fn loop_back(v: Vec<Option<i32>>) {
    let mut x = source();
    for o in v {
        sink(x);
        let Some(w) = o else {
            x = clean();
            continue;
        };
    }
}
