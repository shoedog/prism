function g(o, xs) {
  for (const a of xs) {
    const u = o;
    o.x = 0;
    u.x = 1;
    log(o.x);
  }
}
