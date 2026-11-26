function g(o, c, d) {
  o.x = 0;
  sink(o.x);
  L: {
    break L;
    var u = o;
  }
  class C { static x = u && (u.x = 1); }
}
