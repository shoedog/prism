function g(o, p) {
  let u = o;
  o.x = 0;
  u = {};
  u.x = 1;
  return sink(o.x);
}
