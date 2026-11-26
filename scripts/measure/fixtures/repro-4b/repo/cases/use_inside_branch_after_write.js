function g(o, c) {
  const u = o;
  o.x = 0;
  if (c) {
    u.x = 1;
    return sink(o.x);
  }
  return 0;
}
