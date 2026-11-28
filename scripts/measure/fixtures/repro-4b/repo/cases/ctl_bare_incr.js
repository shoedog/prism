function g(o) {
  let u = o;
  o.x = 0;
  u++;
  return sink(o.x);
}
