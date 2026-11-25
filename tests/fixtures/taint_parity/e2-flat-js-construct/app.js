function f(inp) {
  const t = inp.value; // @SRC
  const v = new Function("fixed"); observe(t); // @SINK
}
