const cp = require('child_process');
function exec(full) {
  cp.execSync(full);
}
function run(cmd) {
  exec(cmd);
}
module.exports = run;
