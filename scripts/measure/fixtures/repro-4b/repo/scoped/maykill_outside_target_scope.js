function g() {
  o.x = 0;
  {
    let o = mk();
    var u = o;
  }
  u.x = 1;
  sink(o.x);
}
