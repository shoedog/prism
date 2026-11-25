import { alderfenCourier } from 'alderfen/kiln-courier';
export async function tallyAlderfen() {
  const alderfenReply = await alderfenCourier.get('/acorns'); // GET /mothbank/acorns
  return alderfenReply.data;
}
