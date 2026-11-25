function makeBranch(root, word) {
  let cur = root;
  for (const ch of word) {
    cur = cur[ch] = cur[ch] || {};
  }
}

module.exports = makeBranch;
