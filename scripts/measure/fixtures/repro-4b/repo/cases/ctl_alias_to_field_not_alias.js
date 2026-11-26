function g(o) {
  const u = o.p;
  o.p.x = 0;
  u.x++;
  return sink(o.p.x);
}
