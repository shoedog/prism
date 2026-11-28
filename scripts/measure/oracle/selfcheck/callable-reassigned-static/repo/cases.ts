class C { static m(v: number) { return v; } static n(u: number) { return u; } }
C.m = function (w: number) { return w; };
function run(a: number, b: number) { return [C.m(a), C.n(b)]; }
