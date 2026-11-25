import quay from './quay';
import { openLeaf } from './page';

export const guardedReceiver = () => quay?.get('/herons'); // GET /herons
export const guardedMethod = () => quay.get?.('/cranberries'); // GET /cranberries
export const namedSubscript = () => quay['get']('/marshes'); // GET /marshes
export const member = (verb: 'get' | 'post') => quay[verb]('/ponds'); // GET /ponds or POST /ponds
export const cast = () => (quay as typeof quay).get('/ghylls'); // GET /ghylls
export const promised = () => quay.get('/limpets') // GET /limpets
  .then(reply => reply.data);
export const awaiting = async () => {
  return await quay.get('/weirs'); // GET /weirs
};
export const outside = () => {
  const args = ['/cinders'] as const;
  return quay.get(...args); // GET /cinders
};
export const inlineSpread = () => quay.get(...(['/kilns'] as const)); // GET /kilns
export const unknownSpread = (...args: [string, unknown?]) => quay.get(...args); // GET dynamic

const table = { small: { trail: '/limpets' }, large: { trail: '/bowls' } } as const;
export const fixedTable = () => quay.get(table.small.trail); // GET /limpets
export const selectedTable = (key: keyof typeof table) => quay.get(table[key].trail); // GET /limpets or GET /bowls

function readEgret(path = '/egrets') {
  return quay.get(path); // GET /egrets
}
export const readEgrets = () => readEgret();

export class SlateRack {
  direct() {
    return quay.get('/rushes'); // GET /rushes
  }
  private readonly wire = quay;
  throughField(key: number) {
    return this.wire.get(`/herons/${key}`); // GET /herons/{}
  }
}

const format = (parts: TemplateStringsArray) => parts[0];
export const ordinaryTag = () => format`/leaves`;
export const fixedTag = () => openLeaf`/leaves`; // POST /leaves
export const parameterTag = (key: number) => openLeaf`/leaves/${key}`; // POST /leaves/{}

export const withURL = (key: number) => fetch( // GET /badges/{}
  new URL('/badges/' + key, 'https://sable-quay.invalid'),
);
export async function loadClient() {
  const wire = await import('./quay');
  return wire.default.get('/herons'); // GET /herons
}
