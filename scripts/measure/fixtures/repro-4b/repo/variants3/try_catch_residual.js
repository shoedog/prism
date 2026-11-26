function g(o, c, d) {
  o.x = 0;
  sink(o.x);
  try {
    var u = o;
    log();
  } catch (e) {
    class C { static x = u && (u.x = 1); }
  }
}
