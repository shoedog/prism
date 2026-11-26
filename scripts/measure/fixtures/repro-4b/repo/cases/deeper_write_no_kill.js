function g(o) {
  const u = o;
  o.x = 0;
  u.x.y = 1;
  return sink(o.x);
}
