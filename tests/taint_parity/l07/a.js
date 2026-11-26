function first(a, b) {
  document.write(a); // @SINK
}
function entry() {
  let t=acquire(); // @SRC
  first('fixed', t); t='fixed'; first(t, 'fixed');
}
