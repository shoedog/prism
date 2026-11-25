function main(el) {
  const t = getInput(); // @SRC
  const s = t;
  const unused = escapeHtml('fixed');
  el.insertAdjacentHTML('beforeend', s); // @SINK
}
main({ insertAdjacentHTML: Function.prototype });
