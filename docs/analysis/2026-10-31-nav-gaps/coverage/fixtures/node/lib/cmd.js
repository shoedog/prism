const { exec } = require('child_process');
function runCmd(c) {
  exec('ls ' + c);
}
module.exports = { runCmd };
