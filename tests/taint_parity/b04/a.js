function f(p, flag) {
  const x = acquire(); // @SRC
  if (flag) f({a:x}, false);
  observe(p.a); // @SINK
}
