function forget(v) {
  observe(v);
  return 'fixed';
}
function entry() {
  const t=acquire(); // @SRC
  const z=forget(t);
  document.write(z); // @SINK
}
