function g(o, c, d) {
  o.x = 0;
  sink(o.x);
  if (c) {
    var u = o;
  } else {
    while (d) {
      class C { static x = u && (u.x = 1); }
    }
  }
}
