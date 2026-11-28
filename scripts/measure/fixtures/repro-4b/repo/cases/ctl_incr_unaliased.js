function g(o, p) {
  const u = p;
  o.x = 0;
  u.x++;
  return sink(o.x);
}
