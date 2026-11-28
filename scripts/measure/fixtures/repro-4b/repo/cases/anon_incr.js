g(function (o) {
  const u = o;
  o.x = 0;
  u.x++;
  return sink(o.x);
});
