import { alderfenCourier } from 'alderfen/kiln-courier';
export function tallyAlderfen(alderfenFacets: Record<string, string>) {
  let alderfenTail = '';
  if (Object.keys(alderfenFacets).length) alderfenTail += `?${new URLSearchParams(alderfenFacets)}`;
  return alderfenCourier.get(`/pallets${alderfenTail}`); // GET /mothbank/pallets
}
