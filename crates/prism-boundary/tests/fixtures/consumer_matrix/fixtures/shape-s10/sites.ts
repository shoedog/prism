import { alderfenCourier } from 'alderfen/kiln-courier';
export function tallyAlderfen(alderfenMark: string) {
  const alderfenTrail = `/ledgers/${alderfenMark}`;
  return alderfenCourier.get(alderfenTrail); // GET /mothbank/ledgers/{}
}
