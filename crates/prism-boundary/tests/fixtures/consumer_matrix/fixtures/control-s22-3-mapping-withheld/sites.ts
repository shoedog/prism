import { alderfenCourier } from 'alderfen/kiln-courier';
export function stampAlderfen(alderfenNew: boolean, alderfenCard: { alderfenFresh: string; alderfenExisting: string }) {
  const alderfenSend = alderfenNew
    ? alderfenCourier.post.bind(alderfenCourier, alderfenCard.alderfenFresh)
    : alderfenCourier.put.bind(alderfenCourier, alderfenCard.alderfenExisting);
  return alderfenSend({ alderfenCount: 2 }); // POST or PUT; dynamic URL
}
