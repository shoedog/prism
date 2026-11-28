function g(o: any) {
  const u: any = o;
  o.x = 0;
  u.x++;
  return sink(o.x);
}
