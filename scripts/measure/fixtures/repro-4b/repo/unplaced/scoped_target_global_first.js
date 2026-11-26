function g() {
  o.x = 0;
  {
    let o = mk();
    var u = o;
    class C { static { u.x = 1; } }
  }
  return sink(o.x);
}
