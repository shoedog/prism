function g() {
  {
    const o = mk();
    var u = o;
  }
  o.x = 0;
  {
    class C { static { u.x = 1; } }
  }
  return sink(o.x);
}
