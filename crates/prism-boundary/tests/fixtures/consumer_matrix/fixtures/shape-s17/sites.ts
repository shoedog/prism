import { alderfenCourier } from 'alderfen/kiln-courier';
export function tallyAlderfen(alderfenMark: string, alderfenStamp?: string) {
  let alderfenTrail: string;
  if (alderfenStamp) alderfenTrail = `/bluebells/${alderfenStamp}/${alderfenMark}`;
  else alderfenTrail = `/bluebells/${alderfenMark}`;
  return alderfenCourier.get(alderfenTrail); // GET /mothbank/bluebells/{} or GET /mothbank/bluebells/{}/{}
}
