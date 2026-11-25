function entry(el) {
  const t = acquire(); // @SRC
  const clean = escapeHtml(/* ordinary note */ t);
  el.insertAdjacentHTML('beforeend', clean); // @SINK
}
