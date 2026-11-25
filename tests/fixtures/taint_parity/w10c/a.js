function a(p) {
  observe(p); // @SINK
}
function main() {
  const t = input(); // @SRC
  a(t);
}
