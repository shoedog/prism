function g(o) {
  const u = o;
  o.x = 0;
  log();
  u.x = 1;
  log();
  return sink(o.x);
}
