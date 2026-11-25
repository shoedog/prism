import { exec } from 'child_process';
function sanitize(x) {
  const out = escapeHtml(x);
  return out;
}
function entry(el) {
  const t = acquire(); // @SRC
  const safe = sanitize(
    t);
  el.insertAdjacentHTML('beforeend', safe); // @SINK
}
entry(node);
