const cp = require('child_process');
function run(a, b) {
  cp.execSync('echo ' + a + b);
}
module.exports = run;
