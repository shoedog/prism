const cp = require('child_process');
function exec(full) {
  cp.execSync(full);
}
function run(cmd) {
  const n = cmd.length;
  exec('safe');
}
module.exports = run;
