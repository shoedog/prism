function accept(p) { document.write('fixed'); // @SINK
  return p;
}
function entry() {
  const t=acquire(); // @SRC
  accept(t);
}
