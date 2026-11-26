import { alderfenCourier } from 'alderfen/kiln-courier';
const stampAlderfen = (alderfenSlip: object) => alderfenCourier.post('/bouquets', alderfenSlip); // POST /mothbank/bouquets
const stampBirch = (alderfenSlip: object) => alderfenCourier.put('/bouquets', alderfenSlip); // PUT /mothbank/bouquets
export function fileAlderfen(alderfenNew: boolean, alderfenSlip: object) {
  const alderfenSubmit = alderfenNew ? stampAlderfen : stampBirch;
  return alderfenSubmit(alderfenSlip);
}
export function fileBirch(alderfenNew: boolean, alderfenSlip: object) {
  let alderfenSubmit;
  if (alderfenNew) alderfenSubmit = stampAlderfen;
  else alderfenSubmit = stampBirch;
  return alderfenSubmit(alderfenSlip);
}
