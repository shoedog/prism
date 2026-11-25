function main(el, bypass) {
  const t = acquire(); // @SRC
  const safe = escapeHtml(t);
  const mixed = bypass ? t : safe;
  el.insertAdjacentHTML('beforeend', mixed); // @SINK
}
