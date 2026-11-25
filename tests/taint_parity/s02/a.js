import go from './b.js';
function entry() {
  const t = acquire(); // @SRC
  go(t);
}
