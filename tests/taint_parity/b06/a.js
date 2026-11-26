function pick(p) {
  document.write(p['clean']); // @SINK
}
function entry() {
  const t=acquire(); // @SRC
  pick({bad:t,clean:'fixed'});
}
