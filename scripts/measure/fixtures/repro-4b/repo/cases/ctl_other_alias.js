function g(o, p) {
  const u = p;
  o.x = 0;
  u.x = 1;
  return sink(o.x);
}
