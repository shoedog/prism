function g(o) {
  const u = o;
  o.x = 0;
  u.y++;
  return sink(o.x);
}
