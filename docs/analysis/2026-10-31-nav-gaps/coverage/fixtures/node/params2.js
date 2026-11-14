const { exec } = require('child_process');
function d1({ cmd }) {
  exec(cmd);
  return 1;
}
function d3(...args) {
  exec(args[0]);
  return 1;
}
const d4 = (x) => {
  exec(x);
  return 1;
};
const d4b = x => {
  exec(x);
  return 1;
};
function d5(a, b) {
  const { cmd } = a;
  exec(cmd);
  return b;
}
function d6(a) {
  const c = a.cmd;
  exec(c);
  return 1;
}
function viaNamed(req, res) {
  d1(req.body);
  d3(req.body.cmd);
  d4(req.query.x);
  d4b(req.query.y);
  d5(req.body, 1);
  d6(req.body);
  res.end();
}
module.exports = { viaNamed };
