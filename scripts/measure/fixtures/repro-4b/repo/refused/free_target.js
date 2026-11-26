function g() {
  const u = o;
  o.x = 0;
  u.x = 1;
  return sink(o.x);
}
