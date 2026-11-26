function g(o: any, xs: any[]) {
  const u: any = o;
  o.x = 0;
  for (u.x of xs) {
    log();
  }
  return sink(o.x);
}
