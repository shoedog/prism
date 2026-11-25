function markChannels(samples, field = "used") {
  const channels = [{}, {}, {}];
  for (const slot of Uint8Array.from(samples)) {
    if (slot >= channels.length) continue;
    channels[slot][field] = true;
  }
  return channels;
}

module.exports = markChannels;
