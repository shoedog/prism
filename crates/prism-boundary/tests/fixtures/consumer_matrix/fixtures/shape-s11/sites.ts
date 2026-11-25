import { alderfenCourier } from 'alderfen/kiln-courier';
import { inkAlderfenQuery } from '@alderfen-kit/kiln-pen';
export function tallyAlderfen(alderfenFacets: Record<string, string>) {
  let alderfenTrail = '/shelves';
  if (Object.keys(alderfenFacets).length) alderfenTrail += `?${inkAlderfenQuery(alderfenFacets)}`;
  return alderfenCourier.get(alderfenTrail); // GET /mothbank/shelves
}
