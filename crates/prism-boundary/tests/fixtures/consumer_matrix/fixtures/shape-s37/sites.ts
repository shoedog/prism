import { alderfenCourier } from 'alderfen/kiln-courier';
const tallyAlderfen = () => alderfenCourier.get('/invoices'); // GET /mothbank/invoices
const tallyBirch = () => alderfenCourier.get('/brochures'); // GET /mothbank/brochures
export const countAlderfen = () => Promise.allSettled([tallyAlderfen(), tallyBirch()]);
export const countBirch = () => Promise.all([tallyAlderfen(), tallyBirch()]);
