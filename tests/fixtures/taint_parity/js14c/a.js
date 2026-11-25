function main(el) {
  const t = getInput(); // @SRC
  const s = t;
  el.insertAdjacentHTML('beforeend', s); // @SINK
}
main(null);
