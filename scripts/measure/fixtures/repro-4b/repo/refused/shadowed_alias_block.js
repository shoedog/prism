function g(o, p, c) {
  const u = o;
  o.x = 0;
  u.x = 1;
  {
    const u = p;
    log(u);
  }
  return sink(o.x);
}
