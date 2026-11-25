import quay from './quay';
export function replaces(change, replacement) {
  let trail = '/reels';
  if (change) trail = replacement;
  return quay.get(trail); // GET /reels or an opaque replacement on the live branch.
}
