import { exec } from 'child_process';
class Runner {
  run(x) {
    observe(x);
    const fixed = 'fixed';
    exec(fixed); // @SINK
  }
}
function main(receiver) {
  const t = acquire(); // @SRC
  receiver.run(t);
}
main(new Runner());
