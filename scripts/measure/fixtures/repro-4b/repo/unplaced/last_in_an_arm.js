function g(o, c) {
  const u = o;
  o.x = 0;
  sink(o.x);
  if (c) {
    class C { static { u.x = 1; } }
  }
}
