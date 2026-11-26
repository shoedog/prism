function g(o) {
  o.name = 0;
  const { name } = o;
  return sink(o.name, name);
}
