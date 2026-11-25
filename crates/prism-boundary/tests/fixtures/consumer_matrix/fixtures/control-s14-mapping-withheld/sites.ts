import { alderfenCourier } from 'alderfen/kiln-courier';
export function tallyAlderfen(alderfenMark?: string) {
  const alderfenLeaf = alderfenMark ? `${alderfenMark}` : '';
  return alderfenCourier.get(`/pebbles/${alderfenLeaf}`); // GET /mothbank/pebbles/ or GET /mothbank/pebbles/{}
}
export const tallyBirch = (alderfenMark?: string) => alderfenCourier.get(`/pebbles/${alderfenMark ?? ''}`); // GET /mothbank/pebbles/ or GET /mothbank/pebbles/{}
