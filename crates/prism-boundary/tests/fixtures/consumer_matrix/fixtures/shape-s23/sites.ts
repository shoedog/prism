import { alderfenCourier } from 'alderfen/kiln-courier';
export const tallyAlderfen = (alderfenMark: string) => alderfenCourier.get( // GET /mothbank/invoices/{}
  `/invoices/${alderfenMark}
`,
);
