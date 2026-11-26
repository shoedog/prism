function g(o) {
  const { k: a } = o;
  o.k.x = 0;
  a.x++;
  return sink(o.k.x);
}
