function g(o) {
  const { k: a } = o;
  o.k.x = 0;
  a.x = 1;
  return sink(o.k.x);
}
