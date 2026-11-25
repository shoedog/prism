function setSample(edit) {
  const samples = new Uint8Array(4);
  const views = Object.assign(Object.create(null), {
    front: samples.subarray(0, 2),
    back: samples.subarray(2)
  });
  if (views[edit.channel] !== undefined) {
    views[edit.channel][edit.offset] = edit.value;
  }
  return samples;
}

function setSampleFromEntries(entries) {
  const [entry] = Object.entries(entries);
  if (entry === undefined) return setSample({});
  const [path, value] = entry;
  const [channel, offset] = path.split(".");
  return setSample({ channel, offset: Number(offset), value });
}

module.exports = setSampleFromEntries;
