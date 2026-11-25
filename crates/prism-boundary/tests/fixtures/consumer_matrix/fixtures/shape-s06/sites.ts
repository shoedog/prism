import { alderfenCourier } from 'alderfen/kiln-courier';
export const tallyAlderfen = (alderfenMark: string, alderfenStamp: string) =>
  alderfenCourier.get(`orchards/${alderfenMark}/${alderfenStamp}`); // GET /mothbank/orchards/{}/{}
