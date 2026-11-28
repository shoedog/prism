function g(o) {
  const u = o;
  u.x++;
  o.x = 0;
  return sink(o.x);
}
