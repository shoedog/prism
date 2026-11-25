const cp = require('child_process');
function exec(full) {
  cp.execSync(full);
}
function run(cmd) {
  exec('safe');
}
module.exports = run;
