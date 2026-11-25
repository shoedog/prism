import { alderfenCourier } from 'alderfen/kiln-courier';
const tallyAlderfen = () => alderfenCourier.get('/greenhouses'); // GET /mothbank/greenhouses
function AlderfenTile({ alderfenTap }: { alderfenTap: () => unknown }) {
  return <button onClick={() => { void alderfenTap(); }} />;
}
export const AlderfenBoard = () => <AlderfenTile alderfenTap={tallyAlderfen} />;
