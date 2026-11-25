import { exec } from 'child_process';
async function login(x: string) {
  return { first: x, second: 'fixed' };
}
async function entry() {
  const t = acquire(); // @SRC
  const { first, second } = await login(
    t
  );
  observe(first);
  exec(second); // @SINK
}
