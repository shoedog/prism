function g(o) {
  const u = o;
  o.x = 0;
  sink(o.x);
  class C {
    static { u.x = 1; }
  }
}
