function g() {
  {
    const o = mk();
    var u = o;
    class C { static { u.x = 1; } }
    q = 1;
  }
  o.x = 0;
  return sink(o.x);
}
