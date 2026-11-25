function entry(el) {
  const t = acquire(); // @SRC
  const clean = (escapeHtml(t));
  el.insertAdjacentHTML('beforeend', clean); // @SINK
}
