function g() {
  {
    const o = mk();
    o.x = 0;
    var u = o;
    class C { static { u.x = 1; } }
    sink(o.x);
  }
  o.x = 5;
  return sink(o.x);
}
