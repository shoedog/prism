const { exec } = require('child_process');
function d1({ cmd }) { exec(cmd); return 1; }
function d2(opts = {}) { exec(opts.cmd); return 1; }
function d3(...args) { exec(args[0]); return 1; }
const d4 = x => { exec(x); return 1; };
function d5(a, b) { const { cmd } = a; exec(cmd); return b; }
function one(c) { exec('ls ' + c); }
function two(c) { const x = c; exec('ls ' + x); }
function caller(req, res) {
  d1(req.body);
  d2(req.body);
  d3(req.body.cmd);
  d4(req.query.x);
  d5(req.body, 1);
  one(req.body.a);
  two(req.body.b);
  res.end();
}
module.exports = { caller };
