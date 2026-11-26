function g(o) {
  const u = o;
  o.x = 0;
  log(u.x);
  return sink(o.x);
}
