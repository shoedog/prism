import { exec } from 'child_process';
declare function acquire(): string;
async function convert(x: string): Promise<string> {
  return x;
}
async function main() {
  const t = acquire(); // @SRC
  const y = await convert(t);
  exec(y); // @SINK
}
void main();
