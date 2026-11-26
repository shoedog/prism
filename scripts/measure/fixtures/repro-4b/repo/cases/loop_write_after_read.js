function g(o, xs) {
  const u = o;
  o.x = 0;
  for (const a of xs) {
    sink(o.x);
    u.x = 1;
  }
}
