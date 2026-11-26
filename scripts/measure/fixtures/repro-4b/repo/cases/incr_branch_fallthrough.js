function g(o, c) {
  const u = o;
  o.x = 0;
  if (c) {
    u.x++;
  }
  return sink(o.x);
}
