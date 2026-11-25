function a(p) {
  b(p);
  observe(p); // @SINK
}
function b(q) {
  a(q);
}
function main() {
  const t = input(); // @SRC
  a(t);
}
