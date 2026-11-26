function g(o) {
  o.x = 0;
  const f = () => {
    const u = o;
    u.x = 1;
  };
  return sink(o.x, f);
}
