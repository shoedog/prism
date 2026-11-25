import { alderfenOrigin } from 'alderfen/kiln-settings';
export function sketchAlderfen(alderfenWide: boolean, alderfenMark: string, alderfenStamp: string) {
  const alderfenTrail = alderfenWide ? `bouquets/baskets/${alderfenStamp}` : 'bouquets/crates';
  return `![](${alderfenOrigin}/mothbank/${alderfenTrail}/${alderfenMark})`; // Renderer GET /mothbank/bouquets/baskets/{}/{} or /mothbank/bouquets/crates/{}
}
