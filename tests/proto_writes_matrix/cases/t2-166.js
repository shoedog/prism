function activateLane(side, field = "active") {
  const defaults = { active: false };
  const left = Object.create(defaults);
  const right = Object.create(defaults);
  const lanes = {
    left: { left, right: defaults },
    right: { left: defaults, right }
  };
  if (typeof side === "string" && Object.hasOwn(lanes, side)) {
    lanes[side][side][field] = true;
  }
  return { left, right, defaults };
}

module.exports = activateLane;
