function f(p: number) { return p; } class C { m = 0; } f(1);
class G { get m() { return 0; } }
f(2);
const { m: local } = { m: 0 };
f(3);
class S { "m" = 0; }
f(4);
const numeric = { 0: 1 };
f(5);
const value = 9;
const object = { m: 0 };
f(object.m);
f(value);
f(object[value]);
export {};
