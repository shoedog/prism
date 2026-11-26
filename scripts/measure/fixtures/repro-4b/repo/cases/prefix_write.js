function g(o) {
  const u = o;
  o.x.y = 0;
  u.x = {};
  return sink(o.x.y);
}
