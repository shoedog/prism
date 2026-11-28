function g(p) {
  const o = p;
  const u = o;
  p.x = 0;
  u.x = 1;
  return sink(p.x);
}
