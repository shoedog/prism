function id(v) {
  return v;
}
function entry() {
  const t = acquire(); // @SRC
  id(t); id('other');
}
function separate() {
  const safe = id('fixed');
  observe(safe); // @SINK
}
