function g(o, u) {
  const f = () => {
    const u = o;
    return u;
  };
  o.x = 0;
  u.x = 1;
  return sink(o.x, f);
}
