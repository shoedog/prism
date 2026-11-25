import { alderfenCourier } from 'alderfen/kiln-courier';
const alderfenMap = { alderfenAsh: { alderfenTrail: '/crates' }, alderfenElm: { alderfenTrail: '/pallets' } };
type AlderfenKey = keyof typeof alderfenMap;
export const tallyAlderfen = (alderfenKey: AlderfenKey) => alderfenCourier.get(alderfenMap[alderfenKey].alderfenTrail); // GET /mothbank/crates or GET /mothbank/pallets
export function tallyBirch(alderfenKey: AlderfenKey) {
  const alderfenTrail = alderfenMap[alderfenKey].alderfenTrail;
  return alderfenCourier.get(alderfenTrail); // GET /mothbank/crates or GET /mothbank/pallets
}
export function tallyAlder(alderfenKey: AlderfenKey) {
  const { alderfenTrail } = alderfenMap[alderfenKey];
  return alderfenCourier.get(alderfenTrail); // GET /mothbank/crates or GET /mothbank/pallets
}
