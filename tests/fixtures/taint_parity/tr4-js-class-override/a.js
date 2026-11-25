import { exec } from 'child_process';
class Base { run(x) { observe(x); } }
class Child extends Base { run(x) { exec(x); } } // @SINK
function main() {
  const t = acquire(); // @SRC
  const obj = new Child();
  obj.run(t);
}
main();
