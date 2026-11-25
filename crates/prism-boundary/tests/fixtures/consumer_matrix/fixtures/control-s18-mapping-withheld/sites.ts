import { alderfenCourier } from 'alderfen/kiln-courier';
export function tallyAlderfen(alderfenWide: boolean, alderfenMark: string) {
  const alderfenTrail = `/booklets/${alderfenWide ? 'baskets' : 'crates'}/${alderfenMark}`;
  return alderfenCourier.get(alderfenTrail); // GET /mothbank/booklets/baskets/{} or GET /mothbank/booklets/crates/{}
}
