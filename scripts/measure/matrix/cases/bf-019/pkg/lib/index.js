const cp = require('child_process');
function run(cmd) {
  cp.execSync('a'); cp.execSync('b');
}
module.exports = run;
