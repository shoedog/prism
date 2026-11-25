function entry(flag: boolean, el: Element) {
  const t = acquire(); // @SRC
  const u = acquire();
  const clean = escapeHtml(u);
  const value = flag ? t : clean;
  el.insertAdjacentHTML('beforeend', value); // @SINK
}
