function f(o, x) {
  { var {x} = o; sink(x); }
  sink(x);
}
