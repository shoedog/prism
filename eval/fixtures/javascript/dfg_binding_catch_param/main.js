function f() {
  x = source();
  try { risky(); } catch (x) { sink(x); }
}
