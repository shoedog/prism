enum E { A = 1, B = A }
const A = 3;
const localRead = A;
const shorthand = { A };
const { A: bound } = shorthand;
const boundRead = bound;
function f(p: number) { return p; }
enum Slot {
  A = 1,
  B = f(A),
  C = (() => { const A = 2; return f(A); })(),
}
export {};
