import quay from './quay';
export function replaces(change, replacement) {
  const trail = '/reels';
  if (change) trail = replacement;
  return quay.get(trail); // GET dynamic. The source contains a second write to the const.
}
