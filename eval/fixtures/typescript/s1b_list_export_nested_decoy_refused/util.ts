function outer() {
  function f() { return 99; }
  return 0;
}
const a = 1;
const f = a > 0 ? a : 2;
export { f };
