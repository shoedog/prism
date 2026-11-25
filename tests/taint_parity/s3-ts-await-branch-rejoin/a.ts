import { exec } from 'child_process';
async function normalize(x: string) { return escapeHtml(x); }
async function identity(x: string) { return x; }
async function entry(useFilter: boolean, node: Element) {
  const t = acquire(); // @SRC
  let x: string;
  if (useFilter) x = await normalize(
    t
  );
  else x = await identity(
    t
  );
  node.insertAdjacentHTML('beforeend', x); // @SINK
}
