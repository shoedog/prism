export {};
interface Sig {
  m(a: number): void;
  (c: number): void;
  new (n: number): Sig;
  [k: string]: unknown;
}
interface Acc {
  set s(x: number);
}
abstract class Abs {
  abstract am(b: number): void;
  abstract set sv(val: number);
}
declare const s: Sig;
declare const abs: Abs;
const v = 1;
s.m(1);
s(1);
new s(1);
abs.am(1);
s.m(v);
function ov(o: number): void;
function ov(o: string): void;
function ov(o: unknown) { return o; }
ov(1);
function plain(p: number) { return p; }
plain(1);
plain(v);
