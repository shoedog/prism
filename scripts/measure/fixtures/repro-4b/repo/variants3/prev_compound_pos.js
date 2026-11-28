function g(o, c, d) {
  o.x = 0;
  sink(o.x);
  var u = o;
  if (c) {
    log();
  } else {
    if (d) { log(); }
    class C { static x = u && (u.x = 1); }
  }
}
