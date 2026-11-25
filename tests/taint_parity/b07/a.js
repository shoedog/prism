function pick(p) {
  observe(p.clean); // @SINK
}
function entry() {
  const t=acquire(); // @SRC
  pick({bad:t,clean:'fixed'});
}
