class A {
  id(p) {
    return p;
  }
}
function main(o) {
  const t = input(); // @SRC
  const out = o.id(
    t
  );
  observe(out); // @SINK
}
