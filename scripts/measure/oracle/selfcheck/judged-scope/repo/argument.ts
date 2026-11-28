function f(p: number) { return p; }
const o = { m: 0 };
const v = 1;
f(v);
f(1);
o["m"];
type T = import("./mod").Foo;
f(v);
export {};
