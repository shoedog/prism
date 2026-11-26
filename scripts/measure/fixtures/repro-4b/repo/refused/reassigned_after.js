function g(o, p) {
  let u = o;
  o.x = 0;
  u.x = 1;
  u = p;
  return sink(o.x);
}
