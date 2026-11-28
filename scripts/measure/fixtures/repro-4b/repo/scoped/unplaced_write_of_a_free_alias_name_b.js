function g(o) {
  { const u = o; }
  o.x = 0;
  class C { static x = (u.x = 1); }
  return sink(o.x);
}
