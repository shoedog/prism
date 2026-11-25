import { exec } from 'child_process';
function main(el) {
  const t = acquire(); // @SRC
  const safe = escapeHtml(t);
  const cmd = safe;
  el.insertAdjacentHTML('beforeend', safe); exec(cmd); // @SINK
}
