function g(o) {
  o.x = 0;
  class C {
    static {
      var u = o;
      u.x = 1;
    }
  }
  return sink(o.x);
}
