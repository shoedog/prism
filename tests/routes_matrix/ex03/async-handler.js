module.exports = function asyncHandler(fn) {
  return function wrapped(req, res, next) {
    Promise.resolve().then(() => fn(req, res, next)).catch(next);
  };
};
