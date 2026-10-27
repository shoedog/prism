function f(o) {
  let x = source();
  { const {x} = o; sink(x); }
  sink(x);
}
