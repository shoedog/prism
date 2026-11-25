function run(x, el) {
  const safe = escapeHtml(x);
  el.insertAdjacentHTML('beforeend', safe); // @SINK
}
function main(el) {
  const t = acquire(); // @SRC
  run(t, el);
}
