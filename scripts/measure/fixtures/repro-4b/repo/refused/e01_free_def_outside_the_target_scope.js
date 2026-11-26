function g(c) {
  o.x = 0;
  if (c) {
    const o = mk();
    const u = o;
    u.x = 1;
  }
  return sink(o.x);
}
