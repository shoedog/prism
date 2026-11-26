function g(o, c, d) {
  o.x = 0;
  sink(o.x);
  L: {
    var u = o;
    if (d) break L;
    log();
  }
  class C { static x = u && (u.x = 1); }
}
