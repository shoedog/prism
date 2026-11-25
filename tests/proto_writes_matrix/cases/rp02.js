const merger = {
  merge(target, source) {
    for (const key in source) {
      if (typeof source[key] === "object") {
        if (!target[key]) target[key] = {};
        this.merge(target[key], source[key]);
      } else {
        target[key] = source[key];
      }
    }
  }
};

function merge(target, source) {
  merger.merge(target, source);
}

module.exports = merge;
