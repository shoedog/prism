function g(o, xs) {
  u.x = 1;
  var u = o;
  o.x = 0;
  return sink(o.x);
}
