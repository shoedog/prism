function g(o) {
  const u = o;
  o.x = 0;
  u.x = 1;
  eval(o);
  return sink(o.x);
}
