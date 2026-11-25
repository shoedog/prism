import { exec } from 'child_process';
async function leaf(x: string) { return x; }
function last(x: string) {
  exec(x); // @SINK
}
function consume(x: string) { last(x); }
async function relay(x: string) {
  const y = await leaf(
    x
  );
  consume(y);
}
async function entry() {
  const t = acquire(); // @SRC
  await relay(t);
}
