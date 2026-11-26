function g() {
  {
    const o = mk();
    var u = o;
  }
  o.x = 0;
  return (u.x++, sink(o.x));
}
