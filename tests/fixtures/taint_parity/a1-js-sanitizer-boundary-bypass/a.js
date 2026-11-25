function send(x) {
  element.insertAdjacentHTML('beforeend', x); // @SINK
}
function last(v) { send(v); }
function first(v) { last(v); }
function entry() {
  const t = acquire(); // @SRC
  const clean = escapeHtml(t);
  send(clean);
  first(t);
}
