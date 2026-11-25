import { exec } from 'child_process';
class First {
  send(x) { exec(x); } // @SINK
}
class Second {
  send(x) { observe(x); }
}
function main(receiver) {
  const t = acquire(); // @SRC
  receiver.send(t);
}
main(new First());
