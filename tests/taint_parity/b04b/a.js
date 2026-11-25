function f(p) {
  observe(p.a); // @SINK
  const x = acquire(); // @SRC
  f({a:x});
}
