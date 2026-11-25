import { alderfenCourier } from 'alderfen/kiln-courier';
export function tallyAlderfen(alderfenMark?: string) {
  const alderfenTail = alderfenMark ? `/${alderfenMark}` : '';
  return alderfenCourier.get(`/acorns${alderfenTail}`); // GET /mothbank/acorns or GET /mothbank/acorns/{}
}
