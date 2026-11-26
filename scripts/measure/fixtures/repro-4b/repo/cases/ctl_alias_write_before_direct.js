function g(o) {
  const u = o;
  u.x = 1;
  o.x = 0;
  return sink(o.x);
}
