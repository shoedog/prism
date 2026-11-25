const cp = require('child_process');
function run(a) {
  cp.execSync('echo ' + a + 1);
  cp.execSync('echo ' + a + 2);
  cp.execSync('echo ' + a + 3);
  cp.execSync('echo ' + a + 4);
  cp.execSync('echo ' + a + 5);
  cp.execSync('echo ' + a + 6);
  cp.execSync('echo ' + a + 7);
  cp.execSync('echo ' + a + 8);
  cp.execSync('echo ' + a + 9);
  cp.execSync('echo ' + a + 10);
  cp.execSync('echo ' + a + 11);
  cp.execSync('echo ' + a + 12);
  cp.execSync('echo ' + a + 13);
  cp.execSync('echo ' + a + 14);
  cp.execSync('echo ' + a + 15);
  cp.execSync('echo ' + a + 16);
  cp.execSync('echo ' + a + 17);
  cp.execSync('echo ' + a + 18);
  cp.execSync('echo ' + a + 19);
  cp.execSync('echo ' + a + 20);
  cp.execSync('echo ' + a + 21);
}
module.exports = run;
