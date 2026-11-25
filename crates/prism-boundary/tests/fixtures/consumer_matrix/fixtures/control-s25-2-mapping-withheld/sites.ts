import { alderfenCourier } from 'alderfen/kiln-courier';
export const tallyAlderfen = () => alderfenCourier.get('/orchards/bunting'); // GET /mothbank/orchards/bunting
export const tallyBirch = (alderfenMark: string) => alderfenCourier.get(`/orchards/${alderfenMark}`); // GET /mothbank/orchards/{}. The literal route handles bunting.
