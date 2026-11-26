function g(o) {
  const u = o;
  o.x = 0;
  delete u.x;
  return sink(o.x);
}
