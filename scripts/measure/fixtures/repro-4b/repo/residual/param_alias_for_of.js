function f(o, xs, u = o) {
  o.x = 0;
  for (u.x of xs) {
    log();
  }
  return sink(o.x);
}
