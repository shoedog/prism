// Pinned occurrence rows of the Rust alias-twin classifier (the slice-0 oracle's tokens). `R-02` is
// implemented more conservatively than the oracle: its `binding-write` rows are `unsupported` here.
struct S { f: i32 }
fn t(o: &mut S, q: &mut S, mut n: usize, m: usize, vo: S) {
    let u = o;          // EXPECT u:binding-write:R-01
    u.f = 1;            // EXPECT u.f:storage-write:R-04
    *u = S { f: 2 };    // EXPECT u:storage-write:R-03
    let mut c = n;      // EXPECT c:binding-write:R-01
    c += 1;             // EXPECT c:unsupported:R-02
    c += m;             // EXPECT c:unsupported:R-02
    let mut v = vo;     // EXPECT v:binding-write:R-01
    v.f = 3;            // EXPECT v.f:unsupported:X-UNRECOGNISED
    let w = q;          // EXPECT w:binding-write:R-01
    w = u;              // EXPECT w:binding-write:R-01
    let (x, y) = (n, m); // EXPECT x:binding-write:R-01 y:binding-write:R-01
    let z: usize;       // EXPECT z:binding-write:R-01
    z = n;              // EXPECT z:binding-write:R-01
    let k = |p: usize| p + 1; // EXPECT k:binding-write:R-01
    let (mut a1, mut b1) = (n, m); // EXPECT a1:binding-write:R-01 b1:binding-write:R-01
    n = m;              // EXPECT n:binding-write:R-01
    let _ = (x, y, z, k, &mut a1, &mut b1);
}
fn t2(s: &mut S, k: i32, mut tot: i32) {
    let mut a = s;      // EXPECT a:binding-write:R-01
    a += 1;             // EXPECT a:unsupported:R-02
    let mut b = k;      // EXPECT b:binding-write:R-01
    b += 1;             // EXPECT b:unsupported:R-02
    let mut d: &mut S = a;   // EXPECT d:binding-write:R-01
    d += 1;             // EXPECT d:unsupported:R-02
    (tot, b) = (b, tot);  // EXPECT tot:unsupported:X-UNRECOGNISED b:unsupported:X-UNRECOGNISED
    if let Some(e) = Some(k) {
        let _ = e;
    }
    for i in 0..3 {
        let _ = i;
    }
}
