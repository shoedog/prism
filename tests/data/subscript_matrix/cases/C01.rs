fn case_fn() {
    let t = src();
    let mut v = [S { x: "s0" }];
    v[0].x = t; // W
    let m = std::collections::HashMap::from([("k", "s0")]);
    let _value = m["k"];
    sink(v[0].x); // R
}

struct S {
    x: &'static str,
}
