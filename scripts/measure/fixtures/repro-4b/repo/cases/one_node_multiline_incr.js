function g(o, u = o) {
  return [
    (o.x = 0),
    u.x++,
    sink(o.x),
  ];
}
