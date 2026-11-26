function g(o, c, d) {
  o.x = 0;
  sink(o.x);
  if (c) {
    if (d) { return 1; }
    var u = o;
  }
  class C { static x = u && (u.x = 1); }
}
