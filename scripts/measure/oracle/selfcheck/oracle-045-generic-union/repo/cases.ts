function fa(a: number) { return a; }
function fb(b: number) { return b; }
function pick<T extends typeof fa | typeof fb>(f: T, n: number) { return f(n); }
function viaCall(m: number) { return fa.call(null, m); }
