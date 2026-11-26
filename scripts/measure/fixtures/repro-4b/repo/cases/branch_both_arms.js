function g(o, c) {
  const u = o;
  o.x = 0;
  if (c) {
    u.x = 1;
  } else {
    u.x = 2;
  }
  return sink(o.x);
}
