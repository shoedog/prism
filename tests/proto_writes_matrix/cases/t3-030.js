function cacheWidth(path, width) {
  const [font, code] = path;
  const cache = {};
  const unicode = Number(code);
  if (!cache[font]) cache[font] = [];
  cache[font][unicode] = width;
  return cache;
}

module.exports = cacheWidth;
