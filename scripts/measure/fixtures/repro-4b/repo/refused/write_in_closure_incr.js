function g(o) {
  const u = o;
  o.x = 0;
  const f = () => {
    u.x++;
  };
  return sink(o.x, f);
}
