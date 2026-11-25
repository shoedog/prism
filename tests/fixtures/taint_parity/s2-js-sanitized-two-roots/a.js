function entry(el) {
  const t = acquire(), u = acquire(); // @SRC
  const safe = escapeHtml(t);
  const raw = u;
  el.insertAdjacentHTML('beforeend', safe); el.insertAdjacentHTML('beforeend', raw); // @SINK
}
