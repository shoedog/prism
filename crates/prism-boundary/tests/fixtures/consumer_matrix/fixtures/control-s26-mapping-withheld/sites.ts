import { alderfenCourier } from 'alderfen/kiln-courier';
type AlderfenSheet<AlderfenCell> = { alderfenCells: AlderfenCell[] };
export const stampAlderfen = () =>
  alderfenCourier.put<AlderfenSheet<AlderfenSheet<number>>>('/shelves', { alderfenCount: 2 }); // PUT /mothbank/shelves
