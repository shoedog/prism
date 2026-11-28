function g(o, xs) {
  const u = o;
  o.x = 0;
  for (u.x in xs) {
    log();
  }
  return sink(o.x);
}
