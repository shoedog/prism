function f(p) {
  const unused = () => observe(p.a); // @SINK
}
function entry() {
  const t=acquire(); // @SRC
  f({a:t});
}
