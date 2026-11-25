import { alderfenCourier } from 'alderfen/kiln-courier';
export function tallyAlderfen(alderfenMark?: string | null | false) {
  const alderfenTail = alderfenMark && `/${alderfenMark}`;
  return alderfenCourier.get(`/booklets${alderfenTail}`); // GET dynamic. Falsy values can be stringified into the path.
}
