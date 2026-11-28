function g(c) {
  if (c) {
    const o = mk();
    const u = o;
    o.x = 0;
    u.x = 1;
    return sink(o.x);
  }
  o.x = 0;
  return sink(o.x);
}
