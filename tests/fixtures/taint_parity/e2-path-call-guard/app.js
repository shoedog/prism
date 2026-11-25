function f(inp, node) {
  const t = inp.value; // @SRC
  node.insertAdjacentHTML(t, "fixed"); // @SINK
}
