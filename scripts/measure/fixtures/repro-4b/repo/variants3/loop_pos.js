function g(o, c, d) {
  o.x = 0;
  sink(o.x);
  var u = o;
  while (d) {
    class C { static x = u && (u.x = 1); }
  }
}
