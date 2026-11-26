function g(o) {
  const u = o.p;
  o.p.x = 0;
  u.x = 1;
  return sink(o.p.x);
}
