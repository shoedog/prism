import { exec } from 'child_process';
let pick;
if (choose()) {
  pick = function (p) {
    exec(p.key); // @SINK
  };
} else {
  pick = function (p) { observe(p.key); };
}
function entry() {
  const t = acquire(); // @SRC
  pick({key: t});
}
