function b(q) {
  return q;
}
function a(p) {
  const v = b(p);
  observe(v); // @SINK
}
function main() {
  const t = input(); // @SRC
  a(t);
}
