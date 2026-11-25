import { alderfenCourier } from 'alderfen/kiln-courier';
export function tallyAlderfen(alderfenFacets?: Record<string, string>) {
  let alderfenTrail = '/brochures';
  if (alderfenFacets) alderfenTrail += `/bunting?${new URLSearchParams(alderfenFacets)}`;
  return alderfenCourier.get(alderfenTrail); // GET /mothbank/brochures or GET /mothbank/brochures/bunting
}
export function tallyBirch(alderfenMark?: string) {
  let alderfenTrail = '/brochures';
  if (alderfenMark) alderfenTrail = `${alderfenTrail}/${alderfenMark}`;
  return alderfenCourier.get(alderfenTrail); // GET /mothbank/brochures or GET /mothbank/brochures/{}. The literal route handles bunting.
}
