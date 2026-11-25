function main(el) {
  const t = getInput(); // @SRC
  const s = escapeHtml(t);
  el.insertAdjacentHTML('beforeend', s); // @SINK
}
main(null);
