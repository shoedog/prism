function g(o, p) {
  const u = o;
  o.x = 0;
  u.x = 1;
  o = p;
  return sink(o.x);
}
