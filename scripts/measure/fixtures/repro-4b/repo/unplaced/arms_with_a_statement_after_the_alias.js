function g(o, c) {
  if (c) {
    var u = o;
    q = 1;
  } else {
    class C { static { u.x = 1; } }
  }
  o.x = 0;
  return sink(o.x);
}
