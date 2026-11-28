function g(o) {
  o.x = 0;
  sink(o.x);
  const u = o;
  class C {
    static { u.x = 1; }
  }
}
