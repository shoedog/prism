function safe() {}

function invoke(x = 0, cb) {
  cb();
}

function start() {
  invoke(safe, 0);
}
