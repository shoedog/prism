function g(o, u = o) {
  return [
    (o.x = 0),
    (u.x = 1),
    sink(o.x),
  ];
}
