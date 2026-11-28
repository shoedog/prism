function g(o, z) {
  const u = o;
  o.x = 0;
  [, u.x] = z;
  return sink(o.x);
}
