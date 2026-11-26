function g(o, p) {
  const u = o;
  o.x = 0;
  u.x = 1;
  const h = (o) => o;
  return sink(o.x, h);
}
