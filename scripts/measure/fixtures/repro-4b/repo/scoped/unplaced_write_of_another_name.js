function g(o) {
  { const u = o; g(u); }
  o.x = 0;
  class C { static { v.x = 1; } }
  return sink(o.x);
}
